/**
 * HTML rendering helpers for reference-html codegen. Pure functions —
 * no I/O, no DOM, no globals. See design §6.3 for the escape contract.
 */
import type { AttributeSpec, ChildrenSpec } from "../registry/defineNode.ts";

const ESCAPE_MAP: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

export function escapeHtml(s: string): string {
  // The regex guarantees `c` is always one of the keys in ESCAPE_MAP, so the
  // lookup is total. The non-null assertion satisfies `noUncheckedIndexedAccess`.
  return s.replace(/[&<>"']/g, (c) => ESCAPE_MAP[c]!);
}

/**
 * Tokenize a small XML snippet for syntax-highlighted display. Output is
 * HTML and goes into element-content position only (never attribute-value
 * position — see design §6.3). All input goes through escapeHtml first.
 */
export function highlightXml(src: string): string {
  const safe = escapeHtml(src);
  // tag-open and tag-close
  // The attrs class allows `&` because escaped attribute values contain
  // `&quot;` entities; the `&gt;` tail anchor still terminates the match
  // correctly because `\s*\/?` cannot consume `&`.
  let out = safe.replace(
    /&lt;(\/?)([A-Za-z][\w]*)((?:\s+.+?)?)(\s*\/?)&gt;/g,
    (
      _m: string,
      slash: string,
      tag: string,
      attrs: string,
      tail: string,
    ): string => {
      const inside = attrs ? highlightAttrs(attrs) : "";
      return (
        `<span class="tk-punct">&lt;${slash}</span>` +
        `<span class="tk-tag">${tag}</span>` +
        inside +
        `<span class="tk-punct">${tail}&gt;</span>`
      );
    },
  );
  // interpolation
  out = out.replace(/\{([^{}]+)\}/g, `<span class="tk-interp">{$1}</span>`);
  return out;
}

function highlightAttrs(s: string): string {
  // Attribute values may contain already-escaped entities (&amp;, &lt;, etc.)
  // — the prior escapeHtml pass converted source `&`, `<`, `>` to entities.
  // The value class matches anything that's not a literal `"` (which only
  // appears as &quot; after escape).
  return s.replace(
    /\s+([\w.-]+)=(&quot;.*?&quot;)/g,
    (_m: string, name: string, value: string): string =>
      ` <span class="tk-attr">${name}</span>=` +
      `<span class="tk-str">${value}</span>`,
  );
}

/** Render the Attributes table for one element page. */
export function attrTable(attrs: Record<string, AttributeSpec>): string {
  const rows = Object.entries(attrs)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([name, spec]) => attrRow(name, spec))
    .join("\n");
  return `<table class="attr-table">
<thead><tr><th>Name</th><th>Type</th><th>Required</th><th>Description</th></tr></thead>
<tbody>
${rows}
</tbody>
</table>`;
}

function attrRow(name: string, spec: AttributeSpec): string {
  const required = spec.required === true;
  const cls = required ? ' class="attr-required"' : "";
  const type = escapeHtml(spec.coerce);
  const doc = escapeHtml(spec.doc ?? "");
  const star = required ? '<span aria-label="required">*</span>' : "";
  return `<tr${cls}><td><code>${name}</code></td><td><code>${type}</code></td><td>${star}</td><td>${doc}</td></tr>`;
}

/**
 * Render the Allowed children block. `pageTags` is the set of element tags
 * that have their own reference page; entries whose tag is not in the set
 * (e.g. child-only specs like <Li>, <Tr>, <Col>, <ChartSeries> that are
 * declared via childAttributeSpecs.ts rather than defineNode) render as
 * <code> without a link.
 */
export function childrenTable(
  children: Record<string, ChildrenSpec>,
  pageTags?: ReadonlySet<string>,
): string {
  const entries = Object.entries(children);
  if (entries.length === 0) {
    return `<p class="ref-empty-children">This element accepts no child elements.</p>`;
  }
  const rows = entries
    .map(([key, spec]) => {
      const tag = spec.element ?? key;
      const card = formatCardinality(spec.min ?? 0, spec.max);
      const doc = escapeHtml(spec.doc ?? "");
      const tagCell =
        pageTags && !pageTags.has(tag)
          ? `<code>&lt;${tag}&gt;</code>`
          : `<a href="../${tag.toLowerCase()}/"><code>&lt;${tag}&gt;</code></a>`;
      return `<tr><td>${tagCell}</td><td>${card}</td><td>${doc}</td></tr>`;
    })
    .join("\n");
  return `<table class="children-table">
<thead><tr><th>Element</th><th>Cardinality</th><th>Description</th></tr></thead>
<tbody>
${rows}
</tbody>
</table>`;
}

// Intentionally duplicated with walkRegistry.ts: dependency direction matters
// (htmlTemplates must not depend on walkRegistry). The helper is tiny and
// stable; see plan §Task 8 "Context" note.
function formatCardinality(min: number, max: number | undefined): string {
  if (max === undefined) return min === 0 ? "0..∞" : `${min}..∞`;
  if (min === max) return String(min);
  return `${min}..${max}`;
}

/** Render the Used by list. */
export function usedByList(
  entries: ReadonlyArray<{ parent: string; cardinality: string }>,
): string {
  if (entries.length === 0) {
    return `<p class="ref-used-empty">Top-level — not nested under another element.</p>`;
  }
  const items = entries
    .map((e) => {
      const slug = e.parent.toLowerCase();
      return `<li><a href="../${slug}/"><code>&lt;${e.parent}&gt;</code></a> <span class="ref-card-cardinality">(${e.cardinality})</span></li>`;
    })
    .join("\n");
  return `<ul class="used-by">${items}</ul>`;
}
