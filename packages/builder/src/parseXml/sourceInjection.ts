// ===== Source-position injection =====
//
// Before feeding an XML string to fast-xml-parser, this utility injects
// `__sourceLine="<N>" __sourceFile="<path>"` attributes into every start tag.
// The parser preserves them as regular attributes (":@" block) which
// convertElement later strips and records into the BuilderSourceMap.
//
// Rationale: fast-xml-parser in preserveOrder mode does not expose node
// positions, so this attribute-injection is the simplest carrier that
// survives imports (<Import>) and template expansion (<Use>).

const TAG_START_RE = /<([A-Za-z_][A-Za-z0-9_:.-]*)(?=[\s/>])/g;

/** Escape an attribute value for inclusion in a double-quoted XML attribute. */
function escapeAttr(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;");
}

/**
 * Inject `__sourceLine`/`__sourceFile` attributes into every start tag of `xml`.
 * Comments (`<!-- ... -->`), CDATA (`<![CDATA[...]]>`), and processing
 * instructions (`<?...?>`) are skipped because the regex only matches when
 * the char after `<` is a letter or underscore.
 */
export function injectSourceAttrs(
  xml: string,
  file: string | undefined,
): string {
  // Build a cumulative newline-offset table so we can map a string index to a
  // 1-based line number in O(log n) via binary search — but the XML sizes we
  // handle are modest, so a linear count over the prefix is fine too.
  const fileAttr = file ? ` __sourceFile="${escapeAttr(file)}"` : "";
  let lastIndex = 0;
  let lineAtLastIndex = 1;

  function lineAt(idx: number): number {
    // Count newlines between lastIndex and idx, then remember the new anchor.
    for (let i = lastIndex; i < idx; i++) {
      if (xml.charCodeAt(i) === 10) lineAtLastIndex++;
    }
    lastIndex = idx;
    return lineAtLastIndex;
  }

  return xml.replace(TAG_START_RE, (match, _tag: string, offset: number) => {
    const line = lineAt(offset);
    return `${match} __sourceLine="${line}"${fileAttr}`;
  });
}
