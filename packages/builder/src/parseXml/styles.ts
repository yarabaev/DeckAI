/**
 * <Styles>/<Style> handling.
 *
 * Styles are collected once at the top of `<SlideGlance>` (or root) and
 * applied to any element that opts in via `class="..."` / `className="..."`.
 */

import {
  type XmlElement,
  formatErrorAt,
  getAttributes,
  getChildElements,
  getTagName,
  parseClassNames,
} from "./xml.ts";

export type StyleRegistry = Record<string, Record<string, string>>;

/**
 * Merge classed-in styles with the element's own attributes. Element-level
 * attributes take precedence. Class/className attributes are stripped from the
 * output. Unknown class names append a non-fatal error.
 */
export function applyStylesToAttrs(
  tagName: string,
  attrs: Record<string, string>,
  styles: StyleRegistry,
  errors: string[],
  errorTagName = tagName,
  node?: XmlElement,
): Record<string, string> {
  const merged: Record<string, string> = {};

  for (const className of parseClassNames(attrs)) {
    const styleAttrs = styles[className];
    if (!styleAttrs) {
      const msg = `<${errorTagName}>: Unknown style class "${className}"`;
      errors.push(node ? formatErrorAt(node, msg) : msg);
      continue;
    }
    Object.assign(merged, styleAttrs);
  }

  for (const [key, value] of Object.entries(attrs)) {
    if (key === "class" || key === "className") continue;
    merged[key] = value;
  }

  return merged;
}

/**
 * Walk root-level children (SlideGlance children, or root children of a
 * Fragment) and collect <Styles>/<Style> declarations. Errors append to the
 * provided array; the registry is returned regardless.
 */
export function collectStyles(
  childElements: XmlElement[],
  errors: string[],
): StyleRegistry {
  const styles: StyleRegistry = {};

  for (const child of childElements) {
    if (getTagName(child) !== "Styles") continue;

    for (const styleEl of getChildElements(child)) {
      const tag = getTagName(styleEl);
      if (tag !== "Style") {
        errors.push(
          formatErrorAt(
            styleEl,
            `Unknown child element <${tag}> inside <Styles>. Expected: <Style>`,
          ),
        );
        continue;
      }

      const attrs = getAttributes(styleEl);
      const name = attrs.name?.trim();

      if (!name) {
        errors.push(
          formatErrorAt(styleEl, '<Style>: Missing required attribute "name"'),
        );
        continue;
      }
      if (styles[name]) {
        errors.push(
          formatErrorAt(styleEl, `<Style>: Duplicate style name "${name}"`),
        );
        continue;
      }
      if (attrs.class !== undefined || attrs.className !== undefined) {
        errors.push(
          formatErrorAt(
            styleEl,
            `<Style>: Style "${name}" cannot use "class" or "className"`,
          ),
        );
      }
      if (getChildElements(styleEl).length > 0) {
        errors.push(
          formatErrorAt(
            styleEl,
            "<Style>: Unexpected child elements. <Style> does not accept child elements",
          ),
        );
      }

      const styleAttrs = { ...attrs };
      delete styleAttrs.name;
      styles[name] = styleAttrs;
    }
  }

  return styles;
}
