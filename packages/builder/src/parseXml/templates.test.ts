import { describe, expect, it } from "vitest";
import { parseBuilderDocument, parseXml, ParseXmlError } from "./parseXml.ts";
import type { Diagnostic } from "../diagnostics.ts";

// parseSlideElement returns the converted single child of <Slide> directly
// (not a wrapper), so result.nodes[0] is the slide's root node.

describe("templates", () => {
  it("expands <Use> with placeholder substitution in text content", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="card" params="title">
            <VStack><Text>{title}</Text></VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="card" title="Hello" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as {
      type: string;
      children: { type: string; text: string }[];
    };
    expect(vstack.type).toBe("vstack");
    expect(vstack.children[0]).toMatchObject({ type: "text", text: "Hello" });
  });

  it("substitutes placeholders in any attribute value (w, gap, alignItems, class)", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="row" params="cardWidth,cardGap,cardAlign,cardClass">
            <VStack w="{cardWidth}" gap="{cardGap}" alignItems="{cardAlign}" class="{cardClass}">
              <Text>x</Text>
            </VStack>
          </Template>
        </Templates>
        <Styles>
          <Style name="extra" backgroundColor="FF0000" />
        </Styles>
        <Slide>
          <Use template="row" cardWidth="200" cardGap="12" cardAlign="center" cardClass="extra" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as Record<string, unknown>;
    expect(vstack.type).toBe("vstack");
    expect(vstack.w).toBe(200);
    expect(vstack.gap).toBe(12);
    expect(vstack.alignItems).toBe("center");
    expect(vstack.backgroundColor).toBe("FF0000");
  });

  it("supports <Slot> for content that does not fit in an attribute", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="card" params="title">
            <VStack>
              <Text>{title}</Text>
              <Slot name="body" />
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="card" title="Heading">
            <Slot name="body">
              <Text>Long content here</Text>
              <Text>Second paragraph</Text>
            </Slot>
          </Use>
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as {
      children: { type: string; text: string }[];
    };
    expect(vstack.children).toHaveLength(3);
    expect(vstack.children[0]).toMatchObject({ type: "text", text: "Heading" });
    expect(vstack.children[1]).toMatchObject({
      type: "text",
      text: "Long content here",
    });
    expect(vstack.children[2]).toMatchObject({
      type: "text",
      text: "Second paragraph",
    });
  });

  it("uses <Slot>'s own children as fallback when not provided", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="card">
            <VStack>
              <Slot name="body"><Text>default body</Text></Slot>
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="card" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as { children: { text: string }[] };
    expect(vstack.children[0]).toMatchObject({
      type: "text",
      text: "default body",
    });
  });

  it("expands templates that nest other templates", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="inner" params="text">
            <Text>{text}</Text>
          </Template>
          <Template name="outer" params="t">
            <VStack>
              <Use template="inner" text="{t}" />
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="outer" t="nested" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as { children: { text: string }[] };
    expect(vstack.children[0]).toMatchObject({ type: "text", text: "nested" });
  });

  it("emits an error when template is not defined", () => {
    const xml = `
      <SlideGlance>
        <Slide>
          <Use template="missing" />
        </Slide>
      </SlideGlance>
    `;
    expect(() => parseBuilderDocument(xml)).toThrow(ParseXmlError);
    try {
      parseBuilderDocument(xml);
    } catch (e) {
      const err = e as ParseXmlError;
      const joined = err.errors.join("\n");
      expect(joined).toContain(
        '<Use template="missing">: template not defined',
      );
    }
  });

  it("emits an error when a placeholder has no matching attribute", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="card"><Text>{title}</Text></Template>
        </Templates>
        <Slide>
          <Use template="card" />
        </Slide>
      </SlideGlance>
    `;
    expect(() => parseBuilderDocument(xml)).toThrow(ParseXmlError);
    try {
      parseBuilderDocument(xml);
    } catch (e) {
      const err = e as ParseXmlError;
      const joined = err.errors.join("\n");
      expect(joined).toContain(
        'placeholder "{title}" has no matching attribute',
      );
    }
  });

  it("non-SlideGlance parseXml() entry leaves the tree unchanged", () => {
    // Templates require <SlideGlance>; standalone parseXml stays simple.
    const xml = `<VStack><Text>plain</Text></VStack>`;
    const nodes = parseXml(xml);
    expect(nodes).toHaveLength(1);
  });

  // T32: __pom* internal attrs must not leak into placeholder params
  it("does not expose __pom* internal attributes as placeholder params even when source tracking is enabled", () => {
    // When trackSourcePos is enabled, the source-injection pass injects
    // __sourceLine / __sourceFile into every start tag, including <Use> elements.
    // Without filtering, these injected attrs would silently satisfy
    // {__sourceLine} placeholders in template bodies, exposing internal metadata.
    //
    // The filter must exclude __pom* attrs from params even with source tracking.
    const xml = `<SlideGlance>
<Templates>
<Template name="spy"><Text>{__sourceLine}</Text></Template>
</Templates>
<Slide><Use template="spy" /></Slide>
</SlideGlance>`;
    // With trackSourcePos (enables source injection), {__sourceLine} must NOT be
    // silently substituted with the injected line number.
    expect(() => parseBuilderDocument(xml, { trackSourcePos: true })).toThrow(
      ParseXmlError,
    );
    try {
      parseBuilderDocument(xml, { trackSourcePos: true });
    } catch (e) {
      const err = e as ParseXmlError;
      expect(err.errors.join("\n")).toContain(
        'placeholder "{__sourceLine}" has no matching attribute',
      );
    }
  });

  // T31: forward reference regression protection
  it("expands a template used before it is defined in the same SlideGlance (forward reference)", () => {
    // <Use template="late"> appears before <Template name="late"> is declared.
    // The single-pass collect→expand model collects all templates first, so
    // forward references must be supported without error.
    const xml = `
      <SlideGlance>
        <Slide>
          <Use template="late" label="world" />
        </Slide>
        <Templates>
          <Template name="late" params="label">
            <Text>{label}</Text>
          </Template>
        </Templates>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const text = result.nodes[0] as { type: string; text: string };
    expect(text.type).toBe("text");
    expect(text.text).toBe("world");
  });

  // T30: {{ escape for literal {
  it("treats {{ as an escape for a literal { character in text content", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="braces">
            <Text>{{value}}</Text>
          </Template>
        </Templates>
        <Slide>
          <Use template="braces" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const text = result.nodes[0] as { type: string; text: string };
    expect(text.type).toBe("text");
    // {{ is the escape for a literal {, so {{value}} → {value}
    expect(text.text).toBe("{value}");
  });

  it("treats {{ as an escape for a literal { in attribute values", () => {
    // Use a numeric attribute (w) so the escaped value can be observed without
    // triggering style-class lookup errors.
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="tmpl" params="size">
            <VStack w="{size}">
              <Text>{{size}}</Text>
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="tmpl" size="200" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as {
      w: number;
      children: { type: string; text: string }[];
    };
    // {size} → "200" (substituted), {{size}} → literal {size} in text
    expect(vstack.w).toBe(200);
    expect(vstack.children[0].text).toBe("{size}");
  });

  it("still substitutes normal {placeholder} when {{ is also present", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="mixed" params="name">
            <Text>Hello {{name}}, you are {name}</Text>
          </Template>
        </Templates>
        <Slide>
          <Use template="mixed" name="World" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const text = result.nodes[0] as { type: string; text: string };
    // {{name}} → literal {name}, {name} → World
    expect(text.text).toBe("Hello {name}, you are World");
  });

  // T33: non-root <Templates> emits TEMPLATES_NOT_AT_ROOT diagnostic
  it("emits TEMPLATES_NOT_AT_ROOT diagnostic when <Templates> is inside a <Slide>", () => {
    const xml = `
      <SlideGlance>
        <Slide>
          <Templates>
            <Template name="inner"><Text>x</Text></Template>
          </Templates>
          <VStack><Text>hello</Text></VStack>
        </Slide>
      </SlideGlance>
    `;
    // Must not throw — non-root <Templates> is a non-fatal diagnostic, not an error.
    const { diagnostics } = parseBuilderDocument(xml);
    const codes = diagnostics.map((d: Diagnostic) => d.code);
    expect(codes).toContain("TEMPLATES_NOT_AT_ROOT");
  });

  it("emits TEMPLATES_NOT_AT_ROOT when <Templates> is nested inside a container", () => {
    const xml = `
      <SlideGlance>
        <Slide>
          <VStack>
            <Templates>
              <Template name="inner"><Text>y</Text></Template>
            </Templates>
            <Text>hello</Text>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    const { diagnostics } = parseBuilderDocument(xml);
    const codes = diagnostics.map((d: Diagnostic) => d.code);
    expect(codes).toContain("TEMPLATES_NOT_AT_ROOT");
  });

  it("does not emit TEMPLATES_NOT_AT_ROOT for a root-level <Templates> block", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="card"><Text>x</Text></Template>
        </Templates>
        <Slide>
          <Use template="card" />
        </Slide>
      </SlideGlance>
    `;
    const { diagnostics } = parseBuilderDocument(xml);
    const codes = diagnostics.map((d: Diagnostic) => d.code);
    expect(codes).not.toContain("TEMPLATES_NOT_AT_ROOT");
  });
});

describe("templates · MyBatis-style directives", () => {
  it("<If> includes its body when test is truthy", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="card" params="show">
            <VStack>
              <If test="show == 'yes'">
                <Text>visible</Text>
              </If>
              <If test="show == 'no'">
                <Text>hidden</Text>
              </If>
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="card" show="yes" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as {
      type: string;
      children: { type: string; text: string }[];
    };
    expect(vstack.children).toHaveLength(1);
    expect(vstack.children[0]).toMatchObject({ type: "text", text: "visible" });
  });

  it("<Choose>/<When>/<Otherwise> picks the first matching branch", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="badge" params="tone">
            <VStack>
              <Choose>
                <When test="tone == 'coral'">
                  <Text>coral-branch</Text>
                </When>
                <When test="tone == 'forest'">
                  <Text>forest-branch</Text>
                </When>
                <Otherwise>
                  <Text>default-branch</Text>
                </Otherwise>
              </Choose>
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="badge" tone="forest" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as {
      children: { text: string }[];
    };
    expect(vstack.children).toHaveLength(1);
    expect(vstack.children[0]?.text).toBe("forest-branch");
  });

  it("<Choose> falls through to <Otherwise> when no <When> matches", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="badge" params="tone">
            <VStack>
              <Choose>
                <When test="tone == 'coral'">
                  <Text>coral</Text>
                </When>
                <Otherwise>
                  <Text>fallback</Text>
                </Otherwise>
              </Choose>
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="badge" tone="mustard" />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as { children: { text: string }[] };
    expect(vstack.children[0]?.text).toBe("fallback");
  });

  it("<Foreach> repeats body once per item with the iteration var bound", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="bullets" params="items">
            <VStack>
              <Foreach items="{items}" as="m">
                <Text>{m.label}</Text>
              </Foreach>
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="bullets" items='[{"label":"alpha"},{"label":"beta"},{"label":"gamma"}]' />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as { children: { text: string }[] };
    expect(vstack.children.map((c) => c.text)).toEqual([
      "alpha",
      "beta",
      "gamma",
    ]);
  });

  it("<Foreach> exposes indexAs/firstAs/lastAs flags", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="timeline" params="items">
            <VStack>
              <Foreach items="{items}" as="m" indexAs="i" firstAs="isFirst" lastAs="isLast">
                <HStack>
                  <Text>{i}-{m}</Text>
                  <If test="!isLast">
                    <Text>connector</Text>
                  </If>
                </HStack>
              </Foreach>
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Use template="timeline" items='["Q1","Q2","Q3"]' />
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as {
      children: { type: string; children: { text: string }[] }[];
    };
    // Three HStack rows, last one skips the connector.
    expect(vstack.children).toHaveLength(3);
    expect(vstack.children[0]?.children.map((c) => c.text)).toEqual([
      "0-Q1",
      "connector",
    ]);
    expect(vstack.children[1]?.children.map((c) => c.text)).toEqual([
      "1-Q2",
      "connector",
    ]);
    expect(vstack.children[2]?.children.map((c) => c.text)).toEqual(["2-Q3"]);
  });

  it("<Foreach> works at the top level with an inline JSON array", () => {
    const xml = `
      <SlideGlance>
        <Slide>
          <VStack>
            <Foreach items='[{"text":"a"},{"text":"b"}]' as="x">
              <Text>{x.text}</Text>
            </Foreach>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as { children: { text: string }[] };
    expect(vstack.children.map((c) => c.text)).toEqual(["a", "b"]);
  });

  it("<Foreach> diagnoses non-array items", () => {
    const xml = `
      <SlideGlance>
        <Slide>
          <VStack>
            <Foreach items='{"not":"array"}' as="m">
              <Text>x</Text>
            </Foreach>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    let threw = false;
    try {
      parseBuilderDocument(xml);
    } catch (e) {
      threw = true;
      expect((e as ParseXmlError).message).toContain("must be a JSON array");
    }
    expect(threw).toBe(true);
  });

  it("<If> diagnoses missing `test` attribute", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="x">
            <VStack><If><Text>hi</Text></If></VStack>
          </Template>
        </Templates>
        <Slide><Use template="x" /></Slide>
      </SlideGlance>
    `;
    let threw = false;
    try {
      parseBuilderDocument(xml);
    } catch (e) {
      threw = true;
      expect((e as ParseXmlError).message).toContain("<If>: missing required");
    }
    expect(threw).toBe(true);
  });

  it("orphaned <When>/<Otherwise> outside <Choose> are reported", () => {
    const xml = `
      <SlideGlance>
        <Slide>
          <VStack>
            <When test="true"><Text>hi</Text></When>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    let threw = false;
    try {
      parseBuilderDocument(xml);
    } catch (e) {
      threw = true;
      expect((e as ParseXmlError).message).toContain(
        "<When> used outside of a <Choose>",
      );
    }
    expect(threw).toBe(true);
  });

  it("dotted-path placeholders walk nested objects within the same Foreach scope", () => {
    const xml = `
      <SlideGlance>
        <Slide>
          <VStack>
            <Foreach items='[{"title":"Hi","tone":{"shade":"coral"}}]' as="m">
              <Text>{m.title}</Text>
              <Text>{m.tone.shade}</Text>
            </Foreach>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as { children: { text: string }[] };
    expect(vstack.children[0]?.text).toBe("Hi");
    expect(vstack.children[1]?.text).toBe("coral");
  });

  it("flatten complex iteration items into scalar Use attributes when crossing template boundaries", () => {
    // Object-valued iteration vars stringify to JSON when passed through a
    // <Use> attribute, which is one-way. Authors should pass scalar fields
    // explicitly: q="{m.q}" tone="{m.tone}" rather than m="{m}".
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="card" params="title,shade">
            <VStack>
              <Text>{title}</Text>
              <Text>{shade}</Text>
            </VStack>
          </Template>
        </Templates>
        <Slide>
          <Foreach items='[{"title":"Hi","tone":{"shade":"coral"}}]' as="m">
            <Use template="card" title="{m.title}" shade="{m.tone.shade}" />
          </Foreach>
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as { children: { text: string }[] };
    expect(vstack.children[0]?.text).toBe("Hi");
    expect(vstack.children[1]?.text).toBe("coral");
  });

  // Regression: a <Use> nested inside a <Foreach> body that itself fills its
  // template's slots via <Slot> children must keep the Slot wrapper through
  // the Foreach's substitute-and-resolve pass. Without the targeted carve-out
  // in substituteAndResolveSlots, the inner Slot would be looked up in the
  // outer (empty) slot dict and fall back to inlining its own children
  // directly under the Use, which then violates "<Use> only accepts <Slot>".
  it("preserves <Slot> wrappers on <Use> children inside a <Foreach> body", () => {
    const xml = `
      <SlideGlance>
        <Templates>
          <Template name="row">
            <HStack>
              <Slot name="left" />
              <Slot name="right" />
            </HStack>
          </Template>
        </Templates>
        <Slide>
          <VStack>
            <Foreach items='[{"l":"alpha","r":"1"},{"l":"beta","r":"2"}]' as="r">
              <Use template="row">
                <Slot name="left"><Text>{r.l}</Text></Slot>
                <Slot name="right"><Text>{r.r}</Text></Slot>
              </Use>
            </Foreach>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as {
      children: { children: { text: string }[] }[];
    };
    expect(vstack.children).toHaveLength(2);
    expect(vstack.children[0]?.children.map((c) => c.text)).toEqual([
      "alpha",
      "1",
    ]);
    expect(vstack.children[1]?.children.map((c) => c.text)).toEqual([
      "beta",
      "2",
    ]);
  });

  // `\n` (literal backslash-n in the XML source) decodes to a real newline
  // inside body text and inside inline runs. `\\n` stays as a literal `\n`
  // string. Unknown escapes (`\X`) survive untouched so paths like
  // `C:\Users\foo` are not mangled.
  it("decodes user-friendly text escapes in body text and inline runs", () => {
    const xml = `
      <SlideGlance>
        <Slide>
          <VStack>
            <Text>line one\\nline two</Text>
            <Text>before <B>bold\\nlines</B> after</Text>
            <Text>literal \\\\n stays</Text>
            <Text>path C:\\Users\\foo</Text>
          </VStack>
        </Slide>
      </SlideGlance>
    `;
    const { document: result } = parseBuilderDocument(xml);
    const vstack = result.nodes[0] as {
      children: { text: string; runs?: { text: string }[] }[];
    };
    expect(vstack.children[0]?.text).toBe("line one\nline two");
    // Inline run inside <B> also carries the decoded newline.
    expect(vstack.children[1]?.runs?.[1]?.text).toBe("bold\nlines");
    expect(vstack.children[2]?.text).toBe("literal \\n stays");
    // Unknown escapes (\U, \f) survive — useful for Windows paths.
    expect(vstack.children[3]?.text).toBe("path C:\\Users\\foo");
  });
});

describe("ParseXmlError messages carry source position when injected", () => {
  // Source attrs are auto-injected when sourcePath is provided, so error
  // messages get a `file:line: ` prefix without the caller having to opt in
  // via trackSourcePos.
  it("prefixes file:line for an unknown tag inside a <Slide> body", () => {
    const xml = `<SlideGlance>
<Slide>
<NotARealTag/>
</Slide>
</SlideGlance>`;
    let threw = false;
    try {
      parseBuilderDocument(xml, { sourcePath: "/tmp/decks/main.sgx" });
    } catch (e) {
      threw = true;
      const err = e as ParseXmlError;
      // The unknown-tag error originates inside parseSlideElement →
      // convertElement, which uses formatErrorAt(child, ...). The message
      // should look like `<file>:<line>: …`.
      expect(err.errors.join("\n")).toMatch(
        /\/tmp\/decks\/main\.sgx:3: Unknown tag: <NotARealTag>/,
      );
    }
    expect(threw).toBe(true);
  });
});
