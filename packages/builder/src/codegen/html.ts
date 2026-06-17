/**
 * HTML reference page generator. Returns a Map of relative path →
 * HTML string consumed by scripts/codegen.ts. See design §5 for page
 * structure and §6 for styling decisions.
 */

import type {
  CompiledNodeDefinition,
  ChildrenSpec,
} from "../registry/defineNode.ts";
import type { CompiledMetaDefinition } from "../registry/defineMeta.ts";
import { buildReferenceModel, type ReferenceModel } from "./walkRegistry.ts";
import {
  escapeHtml,
  highlightXml,
  attrTable,
  childrenTable,
  usedByList,
} from "./htmlTemplates.ts";

const REPO_BLOB =
  "https://github.com/SlideGlance/slideglance/blob/main/packages/builder";

const FOUC_SCRIPT = `<script>(()=>{const s=localStorage.getItem("sg-theme");if(s==="light"||s==="dark")document.documentElement.setAttribute("data-theme",s)})()</script>`;

export function renderElementPage(
  node: CompiledNodeDefinition | CompiledMetaDefinition,
  model: ReferenceModel,
): string {
  const tag = node.tag;
  const example = node.example ? highlightXml(node.example) : "";
  const usedBy = model.usedBy.get(tag) ?? [];
  const seeAlso = model.seeAlso.get(tag) ?? [];
  const loc = model.sourceLocations.get(tag);
  const pageTags = new Set<string>([
    ...model.nodes.map((n) => n.tag),
    ...model.meta.map((m) => m.tag),
  ]);
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>&lt;${tag}&gt; · Builder XML Reference</title>
<link rel="stylesheet" href="../styles.css" />
${FOUC_SCRIPT}
</head>
<body class="ref-page">
<a href="#main" class="skip-link">Skip to content</a>
${renderHeader()}
<div class="ref-shell">
${renderSidebar(tag, model)}
<main id="main" class="ref-content">
<h1><code>&lt;${tag}&gt;</code></h1>
<p class="ref-lede">${escapeHtml(node.description ?? "")}</p>
<h2>Attributes</h2>
${attrTable(node.attributes ?? {})}
<h2>Allowed children</h2>
${childrenTable(getChildrenSpec(node), pageTags)}
<h2>Used by</h2>
${usedByList(usedBy)}
${node.example ? `<h2>Example</h2>\n<pre class="xml-snippet"><code>${example}</code></pre>` : ""}
${
  seeAlso.length > 0
    ? `<h2>See also</h2>\n<ul class="see-also">${seeAlso
        .map(
          (e) =>
            `<li><a href="${escapeHtml(e.href)}">${escapeHtml(e.label)}</a></li>`,
        )
        .join("")}</ul>`
    : ""
}
${loc ? `<h2>Source</h2>\n<p><a href="${REPO_BLOB}/${loc.file}#L${loc.line}"><code>${loc.file}</code> · line ${loc.line}</a></p>` : ""}
</main>
</div>
<script src="../scripts/site.js" defer></script>
</body>
</html>
`;
}

function renderHeader(): string {
  return `<header class="ref-header">
  <a class="brand" href="../">SlideGlance Reference</a>
  <a class="ref-back" href="../../../build/">← Back to Build</a>
</header>`;
}

export function renderIndexPage(model: ReferenceModel): string {
  const nodeCards = model.nodes.map((n) => renderCard(n)).join("");
  const metaCards = model.meta.map((m) => renderCard(m)).join("");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>Builder XML Reference</title>
<link rel="stylesheet" href="./styles.css" />
${FOUC_SCRIPT}
</head>
<body class="ref-page ref-index">
${renderHeader()}
<main class="ref-content">
  <h1>Builder XML Reference</h1>
  <p class="ref-lede">
    <code>${model.namespace}</code> ·
    ${model.nodes.length} nodes · ${model.meta.length} meta ·
    v${escapeHtml(model.packageVersion)}
  </p>
  <label class="ref-filter">
    <input id="ref-q" type="search"
      placeholder="Filter — name, attribute, description..."
      autocomplete="off" spellcheck="false" />
    <kbd aria-hidden="true">/</kbd>
  </label>
  <p class="ref-filter-count" data-count>${model.nodes.length + model.meta.length} of ${model.nodes.length + model.meta.length} elements</p>

  <h2>Visual nodes</h2>
  <div class="ref-grid">${nodeCards}</div>

  <h2>Meta &amp; composition</h2>
  <div class="ref-grid">${metaCards}</div>

  <p class="ref-empty" hidden aria-live="polite">
    No elements match <code data-empty-q></code>.
  </p>
</main>
<script src="./scripts/site.js" defer></script>
</body>
</html>
`;
}

function renderCard(
  node: CompiledNodeDefinition | CompiledMetaDefinition,
): string {
  const slug = node.tag.toLowerCase();
  const attrNames = Object.keys(node.attributes ?? {})
    .join(" ")
    .toLowerCase();
  const haystack = `${node.tag} ${node.description ?? ""} ${attrNames}`
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
  const attrCount = Object.keys(node.attributes ?? {}).length;
  const children = "children" in node ? node.children : {};
  const childCount = Object.keys(children).length;
  const meta = `${attrCount} attrs · ${childCount === 0 ? "no children" : `${childCount} child types`}`;
  return `<a class="ref-card" href="./${slug}/" data-tag="${node.tag}" data-haystack="${escapeHtml(haystack)}">
  <h3><code>&lt;${node.tag}&gt;</code></h3>
  <p class="ref-card-desc">${escapeHtml(node.description ?? "")}</p>
  <p class="ref-card-meta">${meta}</p>
</a>`;
}

/**
 * Visual nodes have `node.children`; meta elements do not. Return a
 * uniform record so childrenTable() can render either input.
 */
function getChildrenSpec(
  node: CompiledNodeDefinition | CompiledMetaDefinition,
): Record<string, ChildrenSpec> {
  if ("children" in node) {
    return node.children;
  }
  // meta — no children field
  return {};
}

function renderSidebar(currentTag: string, model: ReferenceModel): string {
  const link = (tag: string): string => {
    const slug = tag.toLowerCase();
    const aria = tag === currentTag ? ' aria-current="page"' : "";
    return `<li><a href="../${slug}/"${aria}><code>&lt;${tag}&gt;</code></a></li>`;
  };
  return `<nav class="ref-sidebar" aria-label="Builder elements">
<h2>Visual nodes</h2>
<ul>${model.nodes.map((n) => link(n.tag)).join("")}</ul>
<h2>Meta &amp; composition</h2>
<ul>${model.meta.map((m) => link(m.tag)).join("")}</ul>
</nav>`;
}

export function generateReferenceHtml(): Map<string, string> {
  const model = buildReferenceModel();
  const out = new Map<string, string>();

  out.set("index.html", renderIndexPage(model));
  for (const n of [...model.nodes, ...model.meta]) {
    out.set(`${n.tag.toLowerCase()}/index.html`, renderElementPage(n, model));
  }
  out.set("styles.css", REFERENCE_CSS);
  out.set("scripts/site.js", REFERENCE_JS);
  return out;
}

const REFERENCE_CSS = `:root {
  --bg: #f7f5f1;
  --bg-elev: #ffffff;
  --fg: #1f2024;
  --fg-muted: #5f6470;
  --border: #e3ddd2;
  --accent: #c43e1c;
  --accent-hover: #a32d10;
  --ring: rgba(196, 62, 28, 0.32);
  --grid-line: rgba(31, 32, 36, 0.06);
  --code-bg: #1d1f25;
  --code-fg: #f0ece4;
  --code-line: rgba(255, 255, 255, 0.06);
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #15161a; --bg-elev: #1d1f25; --fg: #ececec;
    --fg-muted: #9ca0aa; --border: #2a2d35; --accent: #e85a36;
    --accent-hover: #ff6f48; --ring: rgba(232, 90, 54, 0.4);
    --grid-line: rgba(255, 255, 255, 0.04);
    --code-bg: #0f1014; --code-fg: #e7e2d6;
    --code-line: rgba(255, 255, 255, 0.05);
  }
}
html[data-theme="light"] { --bg: #f7f5f1; --fg: #1f2024; --accent: #c43e1c; --bg-elev: #ffffff; --fg-muted: #5f6470; --border: #e3ddd2; --code-bg: #1d1f25; --code-fg: #f0ece4; }
html[data-theme="dark"] { --bg: #15161a; --fg: #ececec; --accent: #e85a36; --bg-elev: #1d1f25; --fg-muted: #9ca0aa; --border: #2a2d35; --code-bg: #0f1014; --code-fg: #e7e2d6; }
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg); font-family: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
code { font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; font-size: 0.95em; }
.skip-link { position: absolute; left: -9999px; top: 0; padding: 8px 16px; background: var(--accent); color: white; }
.skip-link:focus { left: 0; z-index: 100; }
.ref-header { padding: 16px 32px; display: flex; gap: 24px; align-items: center; border-bottom: 1px solid var(--border); }
.ref-header .brand { font-weight: 600; color: var(--fg); }
.ref-header .ref-back { color: var(--fg-muted); font-size: 14px; }
.ref-shell { display: grid; grid-template-columns: 280px 1fr; max-width: 1280px; margin: 0 auto; gap: 32px; }
.ref-sidebar { position: sticky; top: 0; max-height: 100vh; overflow-y: auto; padding: 32px 16px; }
.ref-sidebar h2 { font-size: 12px; text-transform: uppercase; color: var(--fg-muted); letter-spacing: 0.05em; margin: 16px 0 8px; }
.ref-sidebar ul { list-style: none; padding: 0; margin: 0; }
.ref-sidebar li { margin: 2px 0; }
.ref-sidebar a { display: block; padding: 4px 8px; border-radius: 4px; color: var(--fg); }
.ref-sidebar a:hover { background: var(--grid-line); text-decoration: none; }
.ref-sidebar a[aria-current="page"] { background: var(--accent); color: white; }
.ref-content { padding: 48px 64px; min-width: 0; }
.ref-content h1 { font-size: 36px; margin: 0 0 16px; }
.ref-content h2 { font-size: 20px; margin: 32px 0 12px; border-bottom: 1px solid var(--border); padding-bottom: 4px; }
.ref-lede { color: var(--fg-muted); margin: 0 0 24px; }
.attr-table, .children-table { width: 100%; border-collapse: collapse; font-size: 14px; }
.attr-table th, .attr-table td, .children-table th, .children-table td { padding: 8px 12px; border-bottom: 1px solid var(--border); text-align: left; vertical-align: top; }
.attr-required { background: rgba(196, 62, 28, 0.04); }
.attr-required [aria-label="required"] { color: var(--accent); font-weight: 600; }
.used-by { list-style: none; padding: 0; }
.used-by li { padding: 4px 0; }
.see-also { list-style: none; padding: 0; }
.see-also li { padding: 4px 0; }
.xml-snippet { background: var(--code-bg); color: var(--code-fg); padding: 16px; border-radius: 6px; overflow-x: auto; font-size: 13px; line-height: 1.5; }
.tk-tag { color: #ff8957; }
.tk-attr { color: #f0c674; }
.tk-str { color: #b5bd68; }
.tk-punct { color: #969896; }
.tk-interp { color: #81a2be; }
/* index page */
.ref-index .ref-content { padding: 48px 64px; max-width: 1280px; margin: 0 auto; }
.ref-filter { display: flex; gap: 8px; align-items: center; padding: 12px 16px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-elev); margin: 24px 0 8px; }
.ref-filter input { flex: 1; border: 0; background: transparent; font: inherit; color: var(--fg); outline: none; }
.ref-filter kbd { font-size: 12px; padding: 2px 6px; border: 1px solid var(--border); border-radius: 4px; color: var(--fg-muted); }
.ref-filter-count { color: var(--fg-muted); font-size: 13px; margin: 0 0 24px; }
.ref-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 16px; }
.ref-card { padding: 16px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-elev); color: var(--fg); display: block; }
.ref-card:hover { border-color: var(--accent); text-decoration: none; }
.ref-card[hidden] { display: none; }
.ref-card h3 { margin: 0 0 8px; font-size: 15px; }
.ref-card-desc { margin: 0 0 12px; font-size: 13px; color: var(--fg-muted); line-height: 1.4; }
.ref-card-meta { margin: 0; font-size: 12px; color: var(--fg-muted); }
.ref-empty { color: var(--fg-muted); padding: 24px 0; }
@media (max-width: 900px) {
  .ref-shell { grid-template-columns: 1fr; }
  .ref-sidebar { position: static; max-height: none; }
}
`;

const REFERENCE_JS = `(() => {
  const input = document.getElementById("ref-q");
  if (!input) {
    // No filter — still wire theme toggle if present (not on this page)
    return;
  }
  const cards = Array.from(document.querySelectorAll(".ref-card"));
  const counter = document.querySelector("[data-count]");
  const empty = document.querySelector(".ref-empty");
  const emptyQ = document.querySelector("[data-empty-q]");
  function apply() {
    const q = input.value.trim().toLowerCase();
    let shown = 0;
    for (const c of cards) {
      const m = !q || c.dataset.haystack.includes(q);
      c.hidden = !m;
      if (m) shown++;
    }
    if (counter) counter.textContent = shown + " of " + cards.length + " elements";
    if (empty) empty.hidden = shown > 0 || q === "";
    if (emptyQ && q && shown === 0) emptyQ.textContent = q;
    const url = new URL(location.href);
    if (q) url.searchParams.set("q", q); else url.searchParams.delete("q");
    history.replaceState(null, "", url);
  }
  const initial = new URL(location.href).searchParams.get("q");
  if (initial) { input.value = initial; apply(); }
  input.addEventListener("input", apply);
  document.addEventListener("keydown", (e) => {
    const t = e.target;
    const editable = t && t.matches && t.matches('input, textarea, select, [contenteditable=""], [contenteditable="true"], [role="textbox"]');
    if (e.key === "/" && document.activeElement !== input && !editable) {
      e.preventDefault();
      input.focus();
    }
  });
})();
`;
