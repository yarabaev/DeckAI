/**
 * Wraps a pptxgenjs instance's `write` (and `writeFile`) so callers
 * transparently receive PPTX bytes that have been put through the
 * connector post-process pass. The pptxgenjs side has no notion of
 * `<p:cxnSp>`, so this wrapper is the only place where the rewrite
 * actually happens before user-visible bytes leave the builder.
 *
 * Design choice: monkey-patch the methods on the live instance rather
 * than expose a new buildPptxBuffer helper. This keeps the public
 * surface unchanged — existing callers `(await pptx.write({ ... }))`
 * automatically inherit connector support without code edits.
 */

import { postProcessConnectors } from "./cxnSp.ts";
import type { DiagnosticCollector } from "../../diagnostics.ts";
import type { PptxInstance } from "../types.ts";

/* eslint-disable @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call */

type WriteProps = { outputType?: string; compression?: boolean };
type WriteFileProps = WriteProps & { fileName?: string };

type WriteFn = (props?: WriteProps) => Promise<unknown>;
type WriteFileFn = (props?: WriteFileProps) => Promise<unknown>;

/**
 * Convert a Uint8Array back into one of pptxgenjs' published output
 * shapes. We re-emit only the formats that the public API documents
 * (arraybuffer, base64, binarystring, blob, nodebuffer, uint8array).
 * STREAM is not a real bytes type — we treat it as uint8array.
 */
function convertBytes(
  bytes: Uint8Array,
  outputType: string | undefined,
): unknown {
  switch (outputType) {
    case undefined:
    case "arraybuffer":
      return bytes.buffer.slice(
        bytes.byteOffset,
        bytes.byteOffset + bytes.byteLength,
      );
    case "uint8array":
    case "STREAM":
      return bytes;
    case "nodebuffer":
      // Node's Buffer extends Uint8Array; this is a zero-copy view.
      if (typeof globalThis !== "undefined" && (globalThis as any).Buffer) {
        return (globalThis as any).Buffer.from(
          bytes.buffer,
          bytes.byteOffset,
          bytes.byteLength,
        );
      }
      // Fallback: leave as Uint8Array if Buffer is unavailable. Callers
      // that asked for nodebuffer in a non-Node runtime are broken
      // regardless of the post-process.
      return bytes;
    case "base64": {
      // Encode without intermediate Buffer where possible (browser too).
      if (typeof globalThis !== "undefined" && (globalThis as any).Buffer) {
        return (globalThis as any).Buffer.from(
          bytes.buffer,
          bytes.byteOffset,
          bytes.byteLength,
        ).toString("base64");
      }
      let binary = "";
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i] as number);
      }
      return btoa(binary);
    }
    case "binarystring": {
      let binary = "";
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i] as number);
      }
      return binary;
    }
    case "blob": {
      if (typeof Blob !== "undefined") {
        // Detach into a fresh ArrayBuffer so Blob receives a non-shared
        // BlobPart that the lib.dom typings accept on every TS target.
        const copy = new Uint8Array(bytes.byteLength);
        copy.set(bytes);
        return new Blob([copy.buffer], {
          type: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        });
      }
      return bytes;
    }
    default:
      return bytes;
  }
}

/**
 * Patch the given pptxgenjs instance's `write` method so its result is
 * the connector-rewritten PPTX. Idempotent — calling twice on the
 * same instance leaves only the first wrapping installed (the second
 * call detects the marker and returns).
 *
 * The diagnostics collector is shared with the rest of the builder so
 * any CONNECTOR_UNKNOWN_SHAPE_IDX records produced during rewrite end
 * up on the same `BuildPptxResult.diagnostics` list as parse / layout
 * findings. The post-process is invoked lazily inside the patched
 * `write`, so collector ordering naturally follows call time.
 */
export function wrapPptxWriteWithConnectors(
  pptx: PptxInstance,
  collector: DiagnosticCollector,
): void {
  const inst = pptx as unknown as Record<string, unknown>;
  if (inst.__sgConnectorPatched) return;
  const originalWrite = (inst.write as WriteFn).bind(pptx);
  inst.write = async (props?: WriteProps) => {
    const bytes = (await originalWrite({
      ...(props ?? {}),
      outputType: "uint8array",
    })) as Uint8Array;
    const { bytes: patched, diagnostics } = await postProcessConnectors(bytes);
    for (const d of diagnostics) {
      collector.add(d.code, d.message);
    }
    return convertBytes(patched, props?.outputType);
  };

  // writeFile -> write -> filesystem. Re-implement on top of our
  // wrapped write so connector bytes reach disk too. We follow the
  // same default file name (Presentation.pptx) and .pptx extension
  // semantics pptxgenjs uses internally.
  const originalWriteFile = (inst.writeFile as WriteFileFn | undefined)?.bind(
    pptx,
  );
  if (originalWriteFile) {
    inst.writeFile = async (props?: WriteFileProps) => {
      const rawName = props?.fileName ?? "Presentation.pptx";
      const fileName = rawName.toLowerCase().endsWith(".pptx")
        ? rawName
        : `${rawName}.pptx`;
      const bytes = (await (inst.write as WriteFn)({
        outputType: "nodebuffer",
        compression: props?.compression,
      })) as { buffer?: ArrayBuffer } & Uint8Array;
      // Node / Bun side: write to disk via fs.promises if available.
      // The pptxgenjs original tries `fs.writeFile` first then falls
      // back to a browser download. We mirror the Node branch only —
      // the wrapped path is the canonical Node export route in the
      // builder (the browser writer would also pull post-processed
      // bytes via our `write`).
      try {
        const fsMod = await import("node:fs/promises");
        await fsMod.writeFile(fileName, bytes);
        return fileName;
      } catch {
        // Browser path: fall back to the original implementation which
        // already calls our patched write, so the bytes still include
        // connectors.
        return originalWriteFile(props);
      }
    };
  }

  inst.__sgConnectorPatched = true;
}

/* eslint-enable @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call */
