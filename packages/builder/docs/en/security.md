---
title: "@slideglance/builder — Security"
lang: en
kind: guides
package: builder
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - packages/builder/src/buildPptx.ts
  - packages/builder/src/parseXml/
---

# Security

`@slideglance/builder` is safe to run on trusted XML out of the box. When the XML comes from end users, AI agents, or any other untrusted source, four attribute classes deserve specific hardening.

## Threat model

- **Server-side rendering** of untrusted XML (a SaaS endpoint, a webhook, an LLM agent's output) — the most common case.
- **CLI tools** that build decks from user-supplied input.
- **CI pipelines** that compile decks from PR-provided files.

The risks are file-system reads, server-side request forgery (SSRF), and PPTX-embedded scripted hyperlinks. The package never executes arbitrary code from XML — there is no eval, no scripting, no plugin loading.

## At-risk attributes

| Attribute                 | Risk                                                                         | Mitigation                                                 |
| ------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `<Image src>`             | pptxgenjs reads paths via `fs.readFileSync` and URLs via `https.get`.        | `imageSrcGuard`                                            |
| `<Master backgroundPath>` | Same as `<Image src>`.                                                       | `imageSrcGuard`                                            |
| `<A href>`                | PPTX hyperlinks support `javascript:` / `vbscript:` schemes in some viewers. | `allowedHrefSchemes` allowlist (default: web schemes only) |
| `<Import src>`            | Reads arbitrary files via the caller-supplied resolver.                      | Resolver enforces base directory                           |

## `<Image src>` and `<Master backgroundPath>`

By default, every `src` value is passed through to pptxgenjs, which decides whether to read a local path, fetch a URL, or decode a `data:` URI. For untrusted input, opt in to `imageSrcGuard`:

```ts
import { resolve } from "node:path";
import { buildPptx } from "@slideglance/builder";

await buildPptx(
  xml,
  { w: 1280, h: 720 },
  {
    imageSrcGuard: {
      allowSchemes: ["https:", "data:"], // permitted URL schemes
      allowBaseDir: resolve("./assets"), // file:// + relative paths must resolve here
    },
  },
);
```

Rejections emit `INVALID_IMAGE_SRC` and the image is dropped from the slide; the rest of the deck still builds. With `strict: true`, the rejection upgrades to a `DiagnosticsError`.

When `imageSrcGuard` is omitted, **no validation runs** — explicitly opt in for untrusted input.

## `<A href>`

The default scheme allowlist is `https:`, `http:`, `mailto:`, `tel:`. Hyperlinks with any other scheme emit `INVALID_HREF_SCHEME` and the hyperlink is dropped (the surrounding text is preserved).

To extend the allowlist:

```ts
await buildPptx(
  xml,
  { w: 1280, h: 720 },
  {
    allowedHrefSchemes: ["ftp:", "sftp:"],
  },
);
```

Do **not** add `javascript:`, `vbscript:`, or `data:` to the allowlist when accepting untrusted input.

## `<Import src>`

The caller supplies the resolver. A naive `path.resolve(baseDir, src)` allows `<Import src="../../../etc/passwd"/>`. Cycle detection also depends on path normalization — without `realpathSync`, two case-different paths or a symlink can bypass cycle detection on case-insensitive filesystems (e.g. macOS APFS).

```ts
import { readFileSync, realpathSync } from "node:fs";
import { dirname, resolve, sep } from "node:path";
import { buildPptx, type ImportResolver } from "@slideglance/builder";

const allowedBase = realpathSync(resolve("./slides"));

const resolveImport: ImportResolver = (src, fromPath) => {
  const baseDir = fromPath ? dirname(fromPath) : process.cwd();
  const candidate = resolve(baseDir, src);
  // 1. Normalize symbolic links and case differences.
  const absolute = realpathSync(candidate);
  // 2. Confine to allowed base directory.
  if (!absolute.startsWith(allowedBase + sep)) {
    throw new Error(`Import outside allowed directory: ${src}`);
  }
  return { content: readFileSync(absolute, "utf8"), path: absolute };
};
```

`<Import>` recursion is bounded at depth 16. Cycles are detected via the absolute `path` returned by the resolver and reported as a parse error.

## `masterPptx`

When you accept a PowerPoint template from untrusted input, cap its size and the size of any embedded image:

```ts
await buildPptx(
  xml,
  { w: 1280, h: 720 },
  {
    masterPptx: untrustedPptxBuffer,
    masterPptxLimits: {
      maxBytes: 10 * 1024 * 1024, // 10 MB total
      maxImageBytes: 1 * 1024 * 1024, // 1 MB per embedded image
    },
  },
);
```

Defaults: 50 MB total, 5 MB per image. Oversized buffers emit `MASTER_PPTX_SIZE_LIMIT` and are rejected — the build proceeds without the extracted background.

## Diagnostics may echo input verbatim

`Diagnostic.message` includes user-supplied attribute values verbatim:

> `Cannot convert "secret-token-abc123" to number for attribute fontSize`

When logging or transmitting diagnostics in a multi-tenant server, mask sensitive substrings before persisting or forwarding. Stable `Diagnostic.code` values let you branch without reading `message`:

```ts
for (const d of diagnostics) {
  switch (d.code) {
    case "INVALID_IMAGE_SRC":
    case "INVALID_HREF_SCHEME":
      audit.log(d.code, redact(d.sourcePos));
      break;
    default:
      console.warn(d.code);
  }
}
```

## XML parsing

The builder uses `fast-xml-parser` for parsing. It does not resolve external entities (no XXE risk) and does not execute DOCTYPE references. Documents with DOCTYPE declarations are parsed but the declaration is ignored.

## Resource limits to consider

| Limit                   | Default      | What it protects against                    |
| ----------------------- | ------------ | ------------------------------------------- |
| `maxTemplateNodes`      | 100,000      | `<Use>` expansion bombs.                    |
| `<Import>` depth        | 16           | Recursion via imported files.               |
| `<Use>` recursion depth | 32           | Templates that recursively call each other. |
| `masterPptxLimits`      | 50 MB / 5 MB | Bloated template uploads.                   |

These are constants — `maxTemplateNodes` is the only one a caller can override at build time.

## Recommended profile for untrusted input

```ts
await buildPptx(
  untrustedXml,
  { w: 1280, h: 720 },
  {
    imageSrcGuard: {
      allowSchemes: ["https:", "data:"],
      allowBaseDir: resolve("./caller-controlled-assets"),
    },
    // omit allowedHrefSchemes to keep the default web-only allowlist
    resolveImport: hardenedResolver,
    sourcePath: hardenedResolver.rootPath,
    masterPptxLimits: {
      maxBytes: 10 * 1024 * 1024,
      maxImageBytes: 1 * 1024 * 1024,
    },
    maxTemplateNodes: 50_000,
    strict: true,
  },
);
```

Add request timeouts at the HTTP layer so a malicious deck can't keep `pptxgenjs` waiting on a slow image fetch indefinitely.
