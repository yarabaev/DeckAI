// Equalize-dimensions preprocessor for the @slideglance/builder
// reference deck.
//
// Yoga has no flex/subgrid mechanism for matching the height (or width)
// of equivalent elements across sibling cards. This preprocessor scans
// the inlined chapter XML for sentinel attribute values (`auto`) on
// known templates and replaces them with the worst-case rendered
// dimension across siblings within the surrounding scope.
//
// Supported axes:
//
//   • Height equalization on `<Use template="mood-card">` calls — the
//     `titleH` and `bodyH` parameters become the max measured title /
//     body height across siblings in the same `<HStack>`.
//
//   • Width equalization on inline `<Text w="auto:KEY">` markers —
//     every Text in the scope sharing the same KEY adopts the natural
//     width of the widest one (label columns line up across rows).
//
// The preprocessor only edits the XML string. The runtime builder is
// unaware of `auto`; by the time it sees the document, every sentinel
// has been replaced with a concrete pixel number.

// (No Node-specific imports — equalize operates on raw XML strings, callers
//  load files themselves and feed the text in.)

// =====================================================================
// Equalize-template registry. Each template entry declares which of
// its parameters carry the rendered text plus the typography attrs
// the preprocessor needs to estimate the rendered height. Authors
// add new templates here when they want them to participate in
// equalization.
// =====================================================================

type EqualizeAxis = {
  /** Template parameter that carries the text to measure. */
  textParam: string;
  /** Render fontSize (px). */
  fontSize: number;
  /** lineHeight multiplier. */
  lineHeight: number;
  /** Internal padding the template adds around its content (px). */
  padding: number;
  /** Char-width factor for sans-serif (Inter Display ≈ 0.56). */
  charWidthFactor?: number;
  /** Vertical safety pad added on top of measured height (px). */
  cushion?: number;
};

type EqualizeTemplate = Record<string, EqualizeAxis>;

const EQUALIZE_TEMPLATES: Record<string, EqualizeTemplate> = {
  "mood-card": {
    titleH: {
      textParam: "title",
      fontSize: 22,
      lineHeight: 1.2,
      padding: 20,
      charWidthFactor: 0.56,
      cushion: 4,
    },
    bodyH: {
      textParam: "body",
      fontSize: 12,
      lineHeight: 1.4,
      padding: 20,
      charWidthFactor: 0.56,
      cushion: 4,
    },
  },
  "summary-card": {
    titleH: {
      textParam: "title",
      fontSize: 20,
      lineHeight: 1.3,
      padding: 16,
      charWidthFactor: 0.56,
      cushion: 4,
    },
    bodyH: {
      textParam: "body",
      fontSize: 13,
      lineHeight: 1.35,
      padding: 16,
      charWidthFactor: 0.55,
      cushion: 4,
    },
  },
  // step-eq is the explicit-height variant of `step`. The desc body
  // is rendered at body-muted size. card-canvas adds 16px padding.
  "step-eq": {
    descH: {
      textParam: "desc",
      fontSize: 13,
      lineHeight: 1.35,
      padding: 16,
      charWidthFactor: 0.55,
      cushion: 4,
    },
  },
  // callout-eq reserves a body height. callout surface adds 12px
  // padding on every side.
  "callout-eq": {
    bodyH: {
      textParam: "body",
      fontSize: 13,
      lineHeight: 1.35,
      padding: 12,
      charWidthFactor: 0.55,
      cushion: 6,
    },
  },
  // Pricing cards. taglineH is the only segment that varies between
  // sibling cards; price uses fontSize 44 and is always single-line,
  // features list size is driven by the slot. Padding 24 (card-canvas).
  "pricing-card": {
    taglineH: {
      textParam: "tagline",
      fontSize: 13,
      lineHeight: 1.35,
      padding: 24,
      charWidthFactor: 0.55,
      cushion: 4,
    },
  },
  "pricing-card-featured": {
    taglineH: {
      textParam: "tagline",
      fontSize: 13,
      lineHeight: 1.35,
      padding: 24,
      charWidthFactor: 0.55,
      cushion: 4,
    },
  },
};

// =====================================================================
// Layout constants. The preprocessor computes an approximate column
// width for each card — exact pixel-level fidelity is not required;
// the goal is "every sibling reserves the height of the worst case".
// =====================================================================

const SLIDE_WIDTH_PX = 793;
const SLIDE_GUTTER_PX = 64; // page template lateral padding (32 + 32)
const HSTACK_GAP_PX = 14;

// =====================================================================
// Text-height estimator. Word-aware wrapping with per-character width
// = fontSize × charWidthFactor.
// =====================================================================

function estimateTextHeightPx(
  text: string,
  fontSize: number,
  lineHeight: number,
  columnWidthPx: number,
  charWidthFactor = 0.56,
  cushion = 4,
): number {
  const charWidthPx = fontSize * charWidthFactor;
  const charsPerLine = Math.max(1, Math.floor(columnWidthPx / charWidthPx));
  const words = text.split(/\s+/).filter(Boolean);
  let lines = 0;
  let lineLen = 0;
  for (const word of words) {
    if (lineLen === 0) {
      lineLen = Math.min(word.length, charsPerLine);
      lines = 1;
    } else if (lineLen + 1 + word.length <= charsPerLine) {
      lineLen += 1 + word.length;
    } else {
      lines += 1;
      lineLen = Math.min(word.length, charsPerLine);
    }
  }
  if (lines === 0) lines = 1;
  return Math.ceil(lines * fontSize * lineHeight + cushion);
}

function estimateNaturalWidthPx(
  text: string,
  fontSize: number,
  charWidthFactor = 0.56,
  cushion = 8,
): number {
  const charWidthPx = fontSize * charWidthFactor;
  return Math.ceil(text.length * charWidthPx + cushion);
}

function resolveColumnWidthPx(
  wAttr: string | undefined,
  parentWidthPx: number,
  /** Number of siblings sharing the parent. Used when `w` is "max" or
   *  unspecified — flex-grow distributes available width equally. */
  siblingCount = 1,
): number {
  if (!wAttr || wAttr === "max") {
    return parentWidthPx / Math.max(1, siblingCount);
  }
  if (wAttr.endsWith("%")) {
    const pct = parseFloat(wAttr) / 100;
    return parentWidthPx * pct;
  }
  const n = parseFloat(wAttr);
  if (!Number.isNaN(n)) return n;
  return parentWidthPx / Math.max(1, siblingCount);
}

// =====================================================================
// Tag scanning. The XML files are small enough that a regex-based
// walk works without a full parser. We only need to identify
// <HStack>...</HStack> regions and the top-level <Use> / <Text> tags
// inside each one.
// =====================================================================

type UseHit = {
  start: number;
  end: number;
  raw: string;
  attrs: Record<string, string>;
};

type TextHit = {
  start: number;
  end: number;
  raw: string;
  open: string;
  innerText: string;
  attrs: Record<string, string>;
};

type HStackBlock = {
  bodyStart: number;
  bodyEnd: number;
};

const ATTR_PATTERN = /([a-zA-Z_][a-zA-Z0-9_-]*)\s*=\s*("([^"]*)"|'([^']*)')/g;

function parseAttrs(openTag: string): Record<string, string> {
  const attrs: Record<string, string> = {};
  for (const m of openTag.matchAll(ATTR_PATTERN)) {
    const key = m[1];
    if (!key) continue;
    attrs[key] = m[3] ?? m[4] ?? "";
  }
  return attrs;
}

function rebuildUseTag(
  originalTag: string,
  attrs: Record<string, string>,
): string {
  const selfClosing = /\/\s*>$/.test(originalTag);
  const tagBody = Object.entries(attrs)
    .map(([k, v]) => `${k}="${v.replace(/"/g, "&quot;")}"`)
    .join(" ");
  return `<Use ${tagBody}${selfClosing ? " />" : ">"}`;
}

function rebuildTextOpen(open: string, attrs: Record<string, string>): string {
  const selfClosing = /\/\s*>$/.test(open);
  const body = Object.entries(attrs)
    .map(([k, v]) => `${k}="${v.replace(/"/g, "&quot;")}"`)
    .join(" ");
  return `<Text ${body}${selfClosing ? " />" : ">"}`;
}

function findHStackBlocks(src: string): HStackBlock[] {
  const blocks: HStackBlock[] = [];
  let cursor = 0;
  while (cursor < src.length) {
    const open = src.indexOf("<HStack", cursor);
    if (open < 0) break;
    const gt = src.indexOf(">", open);
    if (gt < 0) break;
    const isSelfClose = src.slice(open, gt + 1).endsWith("/>");
    if (isSelfClose) {
      cursor = gt + 1;
      continue;
    }
    let depth = 1;
    let scan = gt + 1;
    const bodyStart = scan;
    while (scan < src.length && depth > 0) {
      const nextOpen = src.indexOf("<HStack", scan);
      const nextClose = src.indexOf("</HStack>", scan);
      if (nextClose < 0) break;
      if (nextOpen >= 0 && nextOpen < nextClose) {
        const og = src.indexOf(">", nextOpen);
        if (og < 0) break;
        if (!src.slice(nextOpen, og + 1).endsWith("/>")) depth += 1;
        scan = og + 1;
      } else {
        depth -= 1;
        scan = nextClose + "</HStack>".length;
        if (depth === 0) {
          blocks.push({ bodyStart, bodyEnd: nextClose });
          cursor = scan;
          break;
        }
      }
    }
    if (depth > 0) break;
  }
  return blocks;
}

function findUseTagsAtTopLevel(slice: string): UseHit[] {
  const hits: UseHit[] = [];
  let cursor = 0;
  let depth = 0;
  while (cursor < slice.length) {
    if (slice[cursor] !== "<") {
      cursor += 1;
      continue;
    }
    if (slice.startsWith("</", cursor)) {
      const gt = slice.indexOf(">", cursor);
      if (gt < 0) break;
      depth = Math.max(0, depth - 1);
      cursor = gt + 1;
      continue;
    }
    const gt = slice.indexOf(">", cursor);
    if (gt < 0) break;
    const tagText = slice.slice(cursor, gt + 1);
    const isSelfClose = /\/\s*>$/.test(tagText);
    const isUse = /^<Use\b/.test(tagText);
    if (isUse && depth === 0) {
      hits.push({
        start: cursor,
        end: gt + 1,
        raw: tagText,
        attrs: parseAttrs(tagText),
      });
    }
    if (!isSelfClose) depth += 1;
    cursor = gt + 1;
  }
  return hits;
}

function findTextTagsAtTopLevel(slice: string): TextHit[] {
  const hits: TextHit[] = [];
  let cursor = 0;
  let depth = 0;
  while (cursor < slice.length) {
    if (slice[cursor] !== "<") {
      cursor += 1;
      continue;
    }
    if (slice.startsWith("</", cursor)) {
      const gt = slice.indexOf(">", cursor);
      if (gt < 0) break;
      depth = Math.max(0, depth - 1);
      cursor = gt + 1;
      continue;
    }
    const gt = slice.indexOf(">", cursor);
    if (gt < 0) break;
    const tagText = slice.slice(cursor, gt + 1);
    const isSelfClose = /\/\s*>$/.test(tagText);
    const isText = /^<Text\b/.test(tagText);
    if (isText && depth === 0) {
      const attrs = parseAttrs(tagText);
      if (isSelfClose) {
        hits.push({
          start: cursor,
          end: gt + 1,
          raw: tagText,
          open: tagText,
          innerText: attrs.text ?? "",
          attrs,
        });
      } else {
        const closeIdx = slice.indexOf("</Text>", gt + 1);
        if (closeIdx < 0) {
          if (!isSelfClose) depth += 1;
          cursor = gt + 1;
          continue;
        }
        const inner = slice.slice(gt + 1, closeIdx).replace(/<[^>]+>/g, "");
        hits.push({
          start: cursor,
          end: closeIdx + "</Text>".length,
          raw: slice.slice(cursor, closeIdx + "</Text>".length),
          open: tagText,
          innerText: inner.trim(),
          attrs,
        });
        cursor = closeIdx + "</Text>".length;
        continue;
      }
    }
    if (!isSelfClose) depth += 1;
    cursor = gt + 1;
  }
  return hits;
}

// =====================================================================
// Equalize pass — height. Replaces every `auto` height sentinel with a
// concrete pixel value derived from the worst-case sibling text.
// =====================================================================

type Replacement = { start: number; end: number; replacement: string };

function equalizeHeights(
  xml: string,
  options: { logLabel?: string } = {},
): string {
  const blocks = findHStackBlocks(xml);
  if (blocks.length === 0) return xml;

  // Updates accumulate on a per-Use-tag basis so multiple axes
  // (titleH, bodyH, …) collapse into a single replacement and don't
  // overlap when applied right-to-left.
  const updates = new Map<
    string,
    {
      hit: UseHit;
      absStart: number;
      absEnd: number;
      attrs: Record<string, string>;
    }
  >();

  // For cross-template equalization we group hits by AXIS name across
  // every template registered in the preprocessor. Two templates that
  // both expose `bodyH` (e.g. mood-card + summary-card) participate in
  // the same group within an HStack scope, as long as their per-axis
  // measurement params (fontSize / lineHeight / padding) match.
  type AxisGroupEntry = { hit: UseHit; tpl: string; axis: EqualizeAxis };
  for (const block of blocks) {
    const slice = xml.slice(block.bodyStart, block.bodyEnd);
    const hits = findUseTagsAtTopLevel(slice);
    if (hits.length === 0) continue;

    // Build axis-name → hits map. Each entry records the template +
    // the per-axis measurement params so we can compute the height per
    // hit using the right config.
    const byAxis = new Map<string, AxisGroupEntry[]>();
    for (const hit of hits) {
      const tpl = hit.attrs.template;
      if (!tpl || !EQUALIZE_TEMPLATES[tpl]) continue;
      const axes = EQUALIZE_TEMPLATES[tpl];
      for (const [axisName, axisCfg] of Object.entries(axes)) {
        if (hit.attrs[axisName] !== "auto") continue;
        let arr = byAxis.get(axisName);
        if (!arr) {
          arr = [];
          byAxis.set(axisName, arr);
        }
        arr.push({ hit, tpl, axis: axisCfg });
      }
    }
    if (byAxis.size === 0) continue;

    const parentWidthPx = SLIDE_WIDTH_PX - SLIDE_GUTTER_PX;
    const cardCount = hits.length;
    const gapsTotal = HSTACK_GAP_PX * Math.max(0, cardCount - 1);
    const adjustedParent = parentWidthPx - gapsTotal;

    for (const [axisName, entries] of byAxis) {
      const candidates: { entry: AxisGroupEntry; height: number }[] = [];
      for (const entry of entries) {
        const { hit, axis } = entry;
        const text = hit.attrs[axis.textParam] ?? "";
        const wAttr = hit.attrs.w;
        const colWidthPx = resolveColumnWidthPx(
          wAttr,
          adjustedParent,
          cardCount,
        );
        const colInner = Math.max(40, colWidthPx - axis.padding * 2);
        const h = estimateTextHeightPx(
          text,
          axis.fontSize,
          axis.lineHeight,
          colInner,
          axis.charWidthFactor,
          axis.cushion,
        );
        candidates.push({ entry, height: h });
      }
      if (candidates.length === 0) continue;
      const max = candidates.reduce((acc, c) => Math.max(acc, c.height), 0);
      for (const { entry } of candidates) {
        const { hit, tpl } = entry;
        const absStart = block.bodyStart + hit.start;
        const absEnd = block.bodyStart + hit.end;
        const key = `${absStart}:${absEnd}`;
        let upd = updates.get(key);
        if (!upd) {
          upd = { hit, absStart, absEnd, attrs: { ...hit.attrs } };
          updates.set(key, upd);
        }
        upd.attrs[axisName] = String(max);
        if (options.logLabel) {
          console.log(
            `  [equalize:${options.logLabel}] ${tpl}/${axisName} → ${max}px ` +
              `(card: ${(hit.attrs.eyebrow ?? hit.attrs.plan ?? hit.attrs.title ?? "").trim().slice(0, 40)})`,
          );
        }
      }
    }
  }

  const replacements: Replacement[] = [];
  for (const entry of updates.values()) {
    replacements.push({
      start: entry.absStart,
      end: entry.absEnd,
      replacement: rebuildUseTag(entry.hit.raw, entry.attrs),
    });
  }
  return applyReplacements(xml, replacements);
}

// =====================================================================
// Equalize pass — width. Looks for `<Text w="auto:KEY">` markers and
// assigns every same-key element the natural width of the widest
// sibling so columns line up across rows.
// =====================================================================

const CLASS_FONT_HEURISTICS: Record<
  string,
  { fontSize: number; charWidthFactor: number }
> = {
  caption: { fontSize: 11, charWidthFactor: 0.55 },
  body: { fontSize: 13, charWidthFactor: 0.55 },
  "body-ink": { fontSize: 13, charWidthFactor: 0.55 },
  "body-muted": { fontSize: 13, charWidthFactor: 0.55 },
  label: { fontSize: 15, charWidthFactor: 0.55 },
  "title-sm": { fontSize: 18, charWidthFactor: 0.55 },
  "title-md": { fontSize: 20, charWidthFactor: 0.55 },
  "title-lg": { fontSize: 24, charWidthFactor: 0.55 },
  "kw-ink": { fontSize: 12, charWidthFactor: 0.65 }, // mono renders wider
  kw: { fontSize: 12, charWidthFactor: 0.65 },
  kbd: { fontSize: 10, charWidthFactor: 0.65 },
};

function equalizeWidths(
  xml: string,
  options: { logLabel?: string } = {},
): string {
  const blocks = findHStackBlocks(xml);
  if (blocks.length === 0) return xml;

  const replacements: Replacement[] = [];

  for (const block of blocks) {
    const slice = xml.slice(block.bodyStart, block.bodyEnd);
    const textHits = findTextTagsAtTopLevel(slice);
    const byKey = new Map<string, TextHit[]>();
    for (const hit of textHits) {
      const w = hit.attrs.w ?? "";
      const m = w.match(/^auto:([A-Za-z][A-Za-z0-9_-]*)$/);
      if (!m || !m[1]) continue;
      const key = m[1];
      let arr = byKey.get(key);
      if (!arr) {
        arr = [];
        byKey.set(key, arr);
      }
      arr.push(hit);
    }
    if (byKey.size === 0) continue;

    for (const [key, hits] of byKey) {
      let max = 0;
      for (const hit of hits) {
        const cls = (hit.attrs.class ?? "")
          .split(/\s+/)
          .find((c) => c in CLASS_FONT_HEURISTICS);
        const heur = cls ? CLASS_FONT_HEURISTICS[cls] : null;
        const fontSize =
          parseFloat(hit.attrs.fontSize ?? "") || heur?.fontSize || 13;
        const charWidthFactor = heur?.charWidthFactor ?? 0.55;
        const w = estimateNaturalWidthPx(
          hit.innerText,
          fontSize,
          charWidthFactor,
        );
        if (w > max) max = w;
      }
      if (max === 0) continue;
      for (const hit of hits) {
        const newAttrs = { ...hit.attrs, w: String(max) };
        const newOpen = rebuildTextOpen(hit.open, newAttrs);
        const replacement = hit.raw.startsWith(hit.open)
          ? newOpen + hit.raw.slice(hit.open.length)
          : newOpen;
        replacements.push({
          start: block.bodyStart + hit.start,
          end: block.bodyStart + hit.end,
          replacement,
        });
        if (options.logLabel) {
          console.log(
            `  [equalize:${options.logLabel}] Text/w@${key} → ${max}px ` +
              `("${hit.innerText.slice(0, 30)}…")`,
          );
        }
      }
    }
  }

  return applyReplacements(xml, replacements);
}

// =====================================================================
// Equalize pass — table columns. When multiple sibling `<Table>` blocks
// share the same parent and at least one of them declares
// `<Col w="auto:KEY"/>`, every Col with that KEY anywhere in the parent
// scope picks the widest natural width of the cells in that column
// position across all participating tables.
//
// The natural-width estimate uses the longest <Td> body in the column
// at the prevailing Td font (default fontSize 13). A small cushion
// keeps cells from touching the cell border.
// =====================================================================

type TableHit = {
  start: number;
  end: number;
  cols: {
    start: number;
    end: number;
    raw: string;
    attrs: Record<string, string>;
  }[];
  /** Each row: array of <Td> inner text by column index. */
  rowsCellText: string[][];
  /** Default font size from `<Table fontSize="…">` if present. */
  fontSize: number;
};

function findTableSiblingsInScope(src: string): TableHit[][] {
  // Group by their lexical parent: tables whose opening tag sit between
  // the same enclosing element pair count as siblings. For the deck's
  // layout this just means "consecutive <Table> tags inside the same
  // VStack/HStack/page slot".
  const tables: TableHit[] = [];
  let cursor = 0;
  while (cursor < src.length) {
    const open = src.indexOf("<Table", cursor);
    if (open < 0) break;
    const gt = src.indexOf(">", open);
    if (gt < 0) break;
    const isSelfClose = src.slice(open, gt + 1).endsWith("/>");
    if (isSelfClose) {
      cursor = gt + 1;
      continue;
    }
    const close = src.indexOf("</Table>", gt + 1);
    if (close < 0) break;
    const body = src.slice(gt + 1, close);
    const tableAttrs = parseAttrs(src.slice(open, gt + 1));
    const fontSize = parseFloat(tableAttrs.fontSize ?? "") || 13;

    // Cols
    const cols: TableHit["cols"] = [];
    const colMatcher = /<Col\b([^>]*?)\/>/g;
    for (const m of body.matchAll(colMatcher)) {
      const idx = m.index ?? 0;
      const tagAbsStart = gt + 1 + idx;
      const tagAbsEnd = tagAbsStart + m[0].length;
      cols.push({
        start: tagAbsStart,
        end: tagAbsEnd,
        raw: m[0],
        attrs: parseAttrs(m[0]),
      });
    }

    // Rows / cells
    const rowsCellText: string[][] = [];
    const trMatcher = /<Tr\b[^>]*>([\s\S]*?)<\/Tr>/g;
    for (const trMatch of body.matchAll(trMatcher)) {
      const tdMatcher = /<Td\b([^>]*)>([\s\S]*?)<\/Td>|<Td\b([^/>]*)\/>/g;
      const cells: string[] = [];
      const trBody = trMatch[1] ?? "";
      for (const tdMatch of trBody.matchAll(tdMatcher)) {
        if (tdMatch[2] !== undefined) {
          cells.push(tdMatch[2].replace(/<[^>]+>/g, "").trim());
        } else {
          // Self-closing Td: read text attr if any.
          const a = parseAttrs(tdMatch[0]);
          cells.push(a.text ?? "");
        }
      }
      rowsCellText.push(cells);
    }

    tables.push({
      start: open,
      end: close + "</Table>".length,
      cols,
      rowsCellText,
      fontSize,
    });
    cursor = close + "</Table>".length;
  }
  // For now treat every Table in the document as a single scope. The
  // deck's tables sit inside chapter pages so siblings within one
  // chapter share the same parent.
  return tables.length > 0 ? [tables] : [];
}

function equalizeTableColumns(
  xml: string,
  options: { logLabel?: string } = {},
): string {
  const groups = findTableSiblingsInScope(xml);
  if (groups.length === 0) return xml;

  const replacements: Replacement[] = [];

  for (const tables of groups) {
    // Build a cross-table map: for each KEY found on a Col, what column
    // indices in each table participate?
    const keyToParticipants = new Map<
      string,
      { table: TableHit; colIndex: number }[]
    >();
    for (const table of tables) {
      table.cols.forEach((col, idx) => {
        const w = col.attrs.w ?? "";
        const m = w.match(/^auto:([A-Za-z][A-Za-z0-9_-]*)$/);
        if (!m || !m[1]) return;
        const key = m[1];
        let arr = keyToParticipants.get(key);
        if (!arr) {
          arr = [];
          keyToParticipants.set(key, arr);
        }
        arr.push({ table, colIndex: idx });
      });
    }

    for (const [key, parts] of keyToParticipants) {
      let max = 0;
      // Measure the widest cell text in the participating column positions.
      for (const { table, colIndex } of parts) {
        for (const row of table.rowsCellText) {
          const cell = row[colIndex] ?? "";
          if (!cell) continue;
          const w = estimateNaturalWidthPx(cell, table.fontSize, 0.55, 16);
          if (w > max) max = w;
        }
      }
      if (max === 0) continue;
      for (const { table, colIndex } of parts) {
        const col = table.cols[colIndex];
        if (!col) continue;
        const newAttrs = { ...col.attrs, w: String(max) };
        const body = Object.entries(newAttrs)
          .map(([k, v]) => `${k}="${v.replace(/"/g, "&quot;")}"`)
          .join(" ");
        replacements.push({
          start: col.start,
          end: col.end,
          replacement: `<Col ${body} />`,
        });
        if (options.logLabel) {
          console.log(
            `  [equalize:${options.logLabel}] Col/w@${key} → ${max}px ` +
              `(table starting at ${table.start})`,
          );
        }
      }
    }
  }

  return applyReplacements(xml, replacements);
}

// =====================================================================
// Cap-bar auto-measurement.
//
// A `<VStack class="bg-coral" w="3" h="capbar:CLASS"/>` placed next to
// a `<Text class="CLASS">…</Text>` can have its height and margin.top
// computed automatically from the referenced style's typography. The
// preprocessor reads CLASS's fontSize + lineHeight from the styles
// declared anywhere in the inlined XML (typography.xml is the usual
// source), then computes:
//
//   margin.top = round(fontSize × (lineHeight - 1) / 2)   ← top leading
//   bar height = round(fontSize × CAP_HEIGHT_RATIO)        ← cap-baseline
//
// pptxgenjs renders text top-aligned with leading split above + below
// the glyph: the cap-top sits at fontSize × (lineHeight - 1) / 2 from
// the frame top, and the baseline lands one cap-height below. The
// CAP_HEIGHT_RATIO (≈0.78 for Inter Display) describes the visible
// cap-height as a fraction of em. The result: the bar visually spans
// cap-top to baseline at any title size, no per-pixel hand-tuning.
// =====================================================================

// Visual cap-baseline span as a fraction of em. The empirically-fit
// value (~1.05) accounts for pptxgenjs rendering the line at slightly
// taller than nominal-fontSize px in the actual PPTX/PDF output —
// strict cap-height (0.72) and ascent (0.9) both produce a bar that
// reads visually short next to the rendered glyph. The 1.05 figure
// matches the cap-top to baseline distance observed in soffice's PDF
// render of the deck.
const CAP_HEIGHT_RATIO = 1.05;

type StyleEntry = { fontSize?: number; lineHeight?: number };

function collectStylesMap(xml: string): Map<string, StyleEntry> {
  const map = new Map<string, StyleEntry>();
  const styleRe = /<Style\b([^>]*?)\/>/g;
  for (const m of xml.matchAll(styleRe)) {
    const attrs = parseAttrs(m[0]);
    const name = attrs.name;
    if (!name) continue;
    const entry: StyleEntry = {};
    if (attrs.fontSize) {
      const n = parseFloat(attrs.fontSize);
      if (!Number.isNaN(n)) entry.fontSize = n;
    }
    if (attrs.lineHeight) {
      const n = parseFloat(attrs.lineHeight);
      if (!Number.isNaN(n)) entry.lineHeight = n;
    }
    map.set(name, entry);
  }
  return map;
}

function equalizeCapBars(
  xml: string,
  styles: Map<string, StyleEntry>,
  options: { logLabel?: string } = {},
): string {
  // Find every VStack-self-closing or VStack-pair with h="capbar:CLASS".
  // Self-closing `<VStack ... h="capbar:CLASS" />` is the most common form.
  const replacements: Replacement[] = [];
  const tagRe = /<VStack\b([^>]*?)\/>/g;
  for (const m of xml.matchAll(tagRe)) {
    const tagText = m[0];
    const attrs = parseAttrs(tagText);
    const h = attrs.h ?? "";
    const cm = h.match(/^capbar:([A-Za-z][A-Za-z0-9_-]*)$/);
    if (!cm || !cm[1]) continue;
    const cls = cm[1];
    const style = styles.get(cls);
    if (!style?.fontSize) continue;
    // Default lineHeight is 1.0 (tight) for cap-bar measurements. If
    // the referenced style does not declare lineHeight explicitly, the
    // bar's frame collapses to fontSize so the bar matches the visible
    // glyph height without phantom leading.
    const lh = style.lineHeight ?? 1.0;
    const fs = style.fontSize;
    const barH = Math.round(fs * CAP_HEIGHT_RATIO);
    // The bar should sit centered on the cap-baseline span of the
    // rendered glyph. pptxgenjs places the glyph baseline at
    //     fs × (lh + ASCENT_RATIO - 1) / 2 + fs × ASCENT_RATIO
    // approx, with cap-top one cap-height above. Empirically this puts
    // the bar's top at fontSize × ((lineHeight - cap-height-ratio) / 2)
    // — the leftover frame leading split above and below the glyph.
    const frameH = fs * lh;
    const marginTop = Math.max(0, Math.round((frameH - barH) / 2));
    const newAttrs: Record<string, string> = { ...attrs, h: String(barH) };
    if (!("margin.top" in newAttrs) && !attrs.margin) {
      newAttrs["margin.top"] = String(marginTop);
    }
    const tagBody = Object.entries(newAttrs)
      .map(([k, v]) => `${k}="${v.replace(/"/g, "&quot;")}"`)
      .join(" ");
    const replacement = `<VStack ${tagBody} />`;
    const start = m.index ?? 0;
    replacements.push({ start, end: start + tagText.length, replacement });
    if (options.logLabel) {
      console.log(
        `  [equalize:${options.logLabel}] capbar/${cls} → h=${barH} margin.top=${marginTop}`,
      );
    }
  }
  return applyReplacements(xml, replacements);
}

function applyReplacements(xml: string, replacements: Replacement[]): string {
  if (replacements.length === 0) return xml;
  replacements.sort((a, b) => b.start - a.start);
  let out = xml;
  for (const r of replacements) {
    out = out.slice(0, r.start) + r.replacement + out.slice(r.end);
  }
  return out;
}

// =====================================================================
// Public entry. Run height + width equalization in sequence so authors
// can mix both within the same HStack.
// =====================================================================

export function equalizeAll(
  xml: string,
  label = "deck",
  styles: Map<string, StyleEntry> = new Map(),
): string {
  let out = equalizeCapBars(xml, styles, { logLabel: label });
  out = equalizeHeights(out, { logLabel: label });
  out = equalizeWidths(out, { logLabel: label });
  out = equalizeTableColumns(out, { logLabel: label });
  return out;
}

// Re-export for build.ts to pre-load the styles map once.
export { collectStylesMap };
export type { StyleEntry };
