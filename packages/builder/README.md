# @slideglance/builder

Declarative XML → PowerPoint compiler. Describe a deck as XML; `@slideglance/builder` produces a real, editable `.pptx` file with Flexbox-driven layout, schema validation, and AI-friendly grammar.

## What it does

Compiles a small declarative XML grammar into editable `.pptx` files. PowerPoint authoring is imperative and visual; a coding agent or pipeline that needs to emit decks at scale wants the opposite — declarative input, deterministic output, and a vocabulary small enough to reason about. `@slideglance/builder` is that vocabulary.

- **AI-friendly grammar** — the XML element set is small (13 visual nodes + meta elements) and every attribute has a typed schema. LLMs generate it reliably.
- **Flexbox layout** — `<VStack>`, `<HStack>`, `<Layer>` map to [yoga-layout](https://yogalayout.dev/) so position math stays out of the markup.
- **Real PPTX output** — every element compiles to native pptxgenjs primitives. Recipients can edit shapes, text, and tables in PowerPoint.
- **Schema validated** — XSD + JSON Schema generated from the same registry that drives the runtime parser. Editor tooling catches mistakes before render.
- **Composable** — `<Import>`, `<Templates>`, `<Styles>`, `<If>` / `<Choose>` / `<Foreach>` let you split decks across files and reuse fragments.
- **Master slides** — define headers, footers, page numbers, and branding once; apply automatically to every slide.
- **Auto-fit** — overflow protection adjusts row heights, font sizes, and gaps when content exceeds the slide before falling back to uniform scaling.

## Install

```sh
npm i @slideglance/builder
# or
pnpm add @slideglance/builder
```

Requires Node.js ≥ 22.

## Quick start

```ts
import { buildPptx } from "@slideglance/builder";

const xml = `
<SlideGlance>
  <Document size="16:9" />
  <Slide>
    <VStack padding="48" gap="24">
      <Text fontSize="48" bold="true">Quarterly Review</Text>
      <Text fontSize="24" color="666666">Revenue +12% YoY</Text>
    </VStack>
  </Slide>
</SlideGlance>
`;

const { pptx } = await buildPptx(xml, { w: 1280, h: 720 });
await pptx.writeFile({ fileName: "review.pptx" });
```

`buildPptx` returns a [pptxgenjs](https://gitbrent.github.io/PptxGenJS/) instance, so the full pptxgenjs save / write API is available — including `pptx.write({ outputType: "nodebuffer" })` for streaming and `pptx.stream()` for HTTP responses.

## Node vocabulary

| Category    | Tags                                                          |
| ----------- | ------------------------------------------------------------- |
| Containers  | `<VStack>`, `<HStack>`, `<Layer>`                             |
| Text        | `<Text>`, `<Ul>` / `<Ol>` / `<Li>`                            |
| Inline      | `<B>`, `<I>`, `<U>`, `<S>`, `<Mark>`, `<Span>`, `<A>`         |
| Media       | `<Image>`, `<Icon>`, `<Svg>`                                  |
| Tables      | `<Table>` (`<Col>`, `<Tr>`, `<Td>`)                           |
| Graphics    | `<Shape>`, `<Line>`, `<Connector>`, `<Chart>`                 |
| Document    | `<SlideGlance>`, `<Document>`, `<Slide>`, `<Fragment>`        |
| Composition | `<Import>`, `<Templates>` / `<Template>` / `<Use>` / `<Slot>` |
| Styling     | `<Styles>` / `<Style>`, `<Master>`                            |
| Control     | `<If>`, `<Choose>` / `<When>` / `<Otherwise>`, `<Foreach>`    |
| Notes       | `<Notes>`                                                     |

The full attribute reference (auto-generated from the runtime schema) lives in [`reference.md`](./reference.md).

## Documentation

| Document                                         | What it covers                                                                  |
| ------------------------------------------------ | ------------------------------------------------------------------------------- |
| [Getting started](./docs/getting-started.md)     | Step-by-step walkthrough — first slide, first deck, first imported file.        |
| [API reference](./docs/api.md)                   | `buildPptx`, `parseBuilderDocument`, options, types, diagnostics.               |
| [XML reference](./docs/xml-reference.md)         | Hand-curated examples for every visual node and common patterns.                |
| [Layout & styling](./docs/layout-and-styling.md) | Flex containers, sizing, positioning, colors, fonts, decoration, master slides. |
| [Composition](./docs/composition.md)             | `<Import>`, `<Templates>` / `<Use>` / `<Slot>`, `<Styles>`, control-flow tags.  |
| [Text measurement](./docs/text-measurement.md)   | Bundled fonts, OpenType vs heuristic measurement, custom-font behavior.         |
| [Security](./docs/security.md)                   | Hardening notes for processing untrusted XML.                                   |
| [VS Code extension](./docs/vscode-extension.md)  | Live preview, click-to-source, PPTX export from `.sgx` files.                   |
| [Schema reference](./reference.md)               | Auto-generated full attribute reference.                                        |

## XML schema integration

The package ships an XSD (`builder.xsd`, namespace `urn:slideglance:builder:v1`) and a JSON Schema (`builder.schema.json`) for editor tooling.

For VS Code with the [Red Hat XML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-xml):

```jsonc
// .vscode/settings.json
{
  "xml.fileAssociations": [
    {
      "pattern": "**/*.sgx",
      "systemId": "./node_modules/@slideglance/builder/builder.xsd",
    },
  ],
}
```

Or annotate the document directly:

```xml
<SlideGlance
  xmlns="urn:slideglance:builder:v1"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="urn:slideglance:builder:v1">
  <Document size="16:9" />
  <Slide><Text>Hello</Text></Slide>
</SlideGlance>
```

The runtime parser does not require the namespace — declaring it only enables editor tooling.

## Reference deck

[`examples/builder-reference`] is a runnable showcase that exercises every node type and composition feature. Use it as a working example or smoke test.
