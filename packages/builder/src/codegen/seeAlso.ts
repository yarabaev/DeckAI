/**
 * Hand-curated "See also" link table — maps registry tags to related
 * documentation pages. Keys must match actual registry tag names; missing
 * keys cause the page to skip the "See also" section. Validated by
 * seeAlso.test.ts. External GitHub URLs cannot be 404-checked offline
 * (see design §4.3).
 */

export interface SeeAlsoEntry {
  readonly label: string;
  readonly href: string;
}

const REPO = "https://github.com/SlideGlance/slideglance/blob/main";
const BUILDER_DOCS = `${REPO}/packages/builder/docs/en`;

export const SEE_ALSO: Readonly<Record<string, ReadonlyArray<SeeAlsoEntry>>> = {
  // Visual nodes
  Text: [
    {
      label: "Inline formatting (B, I, U, S, Mark, Span, A)",
      href: `${BUILDER_DOCS}/xml-reference.md`,
    },
    {
      label: "Text measurement (opentype vs fallback)",
      href: `${BUILDER_DOCS}/text-measurement.md`,
    },
  ],
  Ul: [
    {
      label: "XML reference — Lists",
      href: `${BUILDER_DOCS}/xml-reference.md`,
    },
  ],
  Ol: [
    {
      label: "XML reference — Lists",
      href: `${BUILDER_DOCS}/xml-reference.md`,
    },
  ],
  Image: [
    {
      label: "XML reference — Image",
      href: `${BUILDER_DOCS}/xml-reference.md`,
    },
  ],
  Icon: [
    { label: "XML reference — Icon", href: `${BUILDER_DOCS}/xml-reference.md` },
  ],
  Svg: [
    { label: "XML reference — Svg", href: `${BUILDER_DOCS}/xml-reference.md` },
  ],
  Table: [
    {
      label: "XML reference — Table",
      href: `${BUILDER_DOCS}/xml-reference.md`,
    },
  ],
  Shape: [
    {
      label: "XML reference — Shape",
      href: `${BUILDER_DOCS}/xml-reference.md`,
    },
  ],
  Chart: [
    {
      label: "XML reference — Chart",
      href: `${BUILDER_DOCS}/xml-reference.md`,
    },
  ],
  Line: [
    { label: "XML reference — Line", href: `${BUILDER_DOCS}/xml-reference.md` },
  ],
  VStack: [
    {
      label: "Layout & styling",
      href: `${BUILDER_DOCS}/layout-and-styling.md`,
    },
  ],
  HStack: [
    {
      label: "Layout & styling",
      href: `${BUILDER_DOCS}/layout-and-styling.md`,
    },
  ],
  Layer: [
    {
      label: "Layout & styling",
      href: `${BUILDER_DOCS}/layout-and-styling.md`,
    },
  ],

  // Meta / composition / control flow
  Document: [
    {
      label: "Layout & styling",
      href: `${BUILDER_DOCS}/layout-and-styling.md`,
    },
  ],
  Styles: [
    {
      label: "Layout & styling",
      href: `${BUILDER_DOCS}/layout-and-styling.md`,
    },
  ],
  Style: [
    {
      label: "Layout & styling",
      href: `${BUILDER_DOCS}/layout-and-styling.md`,
    },
  ],
  Templates: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  Template: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  Use: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  Slot: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  Import: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  If: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  Choose: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  When: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  Otherwise: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
  Foreach: [{ label: "Composition", href: `${BUILDER_DOCS}/composition.md` }],
};
