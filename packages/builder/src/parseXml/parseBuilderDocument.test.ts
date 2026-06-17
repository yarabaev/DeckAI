import { describe, expect, it } from "vitest";
import { parseBuilderDocument, parseXml } from "./parseXml.ts";

describe("parseBuilderDocument", () => {
  it("Presentation から slideSize, masters, defaultMaster を抽出する", () => {
    const xml = `
      <SlideGlance><Document size="4:3" defaultMaster="PRIMARY" fontFamily="Pretendard" fontSize="20" color="334155" bold="true" lineHeight="1.5" />
        <Master name="PRIMARY" backgroundColor="F8FAFC" margin="48">
          <MasterRect x="0" y="0" w="1024" h="40" fill.color="0F172A" />
          <MasterText x="48" y="12" w="200" h="28" fontSize="14" color="FFFFFF">Company Name</MasterText>
          <SlideNumber x="900" y="730" fontSize="10" color="1E293B" />
        </Master>
        <Master name="ALT">
          <MasterLine x="0" y="40" w="1024" h="0" line.color="CBD5E1" />
        </Master>
        <VStack w="100%" h="max" padding="48">
          <Text>Default master slide</Text>
        </VStack>
        <VStack master="ALT" w="100%" h="max" padding="48">
          <Text>Alt master slide</Text>
        </VStack>
      </SlideGlance>
    `;

    const { document } = parseBuilderDocument(xml);

    expect(document.slideSize).toEqual({ w: 1024, h: 768 });
    expect(document.defaultMaster).toBe("PRIMARY");
    expect(document.defaultTextStyle).toEqual({
      fontFamily: "Pretendard",
      fontSize: 20,
      color: "334155",
      bold: true,
      lineHeight: 1.5,
    });
    expect(document.masters).toEqual([
      {
        title: "PRIMARY",
        background: { color: "F8FAFC" },
        margin: 48,
        objects: [
          {
            type: "rect",
            x: 0,
            y: 0,
            w: 1024,
            h: 40,
            fill: { color: "0F172A" },
          },
          {
            type: "text",
            text: "Company Name",
            x: 48,
            y: 12,
            w: 200,
            h: 28,
            fontSize: 14,
            color: "FFFFFF",
          },
        ],
        slideNumber: {
          x: 900,
          y: 730,
          fontSize: 10,
          color: "1E293B",
        },
      },
      {
        title: "ALT",
        objects: [
          {
            type: "line",
            x: 0,
            y: 40,
            w: 1024,
            h: 0,
            line: { color: "CBD5E1" },
          },
        ],
      },
    ]);
    expect(document.nodes).toHaveLength(2);
    expect(document.nodes[1]?.master).toBe("ALT");
  });

  it("Master は 일반 노드 자식도 유지해서 후속 렌더링에 사용할 수 있다", () => {
    const { document } = parseBuilderDocument(`
      <SlideGlance><Document defaultMaster="PRIMARY" />
        <Master name="PRIMARY">
          <VStack w="100%" h="max" padding="48">
            <Text fontSize="24">Header</Text>
          </VStack>
        </Master>
        <VStack><Text>Slide</Text></VStack>
      </SlideGlance>
    `);

    expect(document.masters).toEqual([{ title: "PRIMARY" }]);
    expect(document.masterContents?.PRIMARY).toHaveLength(1);
    expect(document.masterContents?.PRIMARY?.[0]).toMatchObject({
      type: "vstack",
      children: [{ type: "text", text: "Header", fontSize: 24 }],
    });
  });

  it("parseXml でも Presentation からスライドノードだけを返す", () => {
    const nodes = parseXml(`
      <SlideGlance><Document defaultMaster="PRIMARY" />
        <Master name="PRIMARY">
          <MasterText x="0" y="0" w="100" h="20">Header</MasterText>
        </Master>
        <VStack master="PRIMARY"><Text>A</Text></VStack>
      </SlideGlance>
    `);

    expect(nodes).toHaveLength(1);
    expect(nodes[0]?.type).toBe("vstack");
    expect(nodes[0]?.master).toBe("PRIMARY");
  });

  it("<Slide> 컨테이너로 감싼 슬라이드를 파싱할 수 있다", () => {
    const { document } = parseBuilderDocument(`
      <SlideGlance><Document defaultMaster="PRIMARY" />
        <Master name="PRIMARY">
          <MasterText x="0" y="0" w="100" h="20">Header</MasterText>
        </Master>
        <Slide>
          <VStack padding="48">
            <Text>A</Text>
          </VStack>
        </Slide>
        <Slide master="ALT">
          <Layer>
            <Text x="10" y="20">B</Text>
          </Layer>
        </Slide>
      </SlideGlance>
    `);

    expect(document.nodes).toHaveLength(2);
    expect(document.nodes[0]).toMatchObject({
      type: "vstack",
      padding: 48,
    });
    expect(document.nodes[1]).toMatchObject({
      type: "layer",
      master: "ALT",
    });
  });

  it("<Slide master> 는 자식 루트 노드에 master 를 주입한다", () => {
    const nodes = parseXml(`
      <SlideGlance>
        <Slide master="REPORT">
          <Notes>speaker note</Notes>
          <VStack>
            <Text>Hello</Text>
          </VStack>
        </Slide>
      </SlideGlance>
    `);

    expect(nodes).toEqual([
      {
        type: "vstack",
        master: "REPORT",
        notes: "speaker note",
        children: [{ type: "text", text: "Hello" }],
      },
    ]);
  });

  it("<Slide> 는 정확히 하나의 자식 슬라이드 루트 노드를 요구한다", () => {
    expect(() =>
      parseBuilderDocument(`
        <SlideGlance>
          <Slide>
            <VStack><Text>A</Text></VStack>
            <VStack><Text>B</Text></VStack>
          </Slide>
        </SlideGlance>
      `),
    ).toThrow(/Expected exactly 1 child slide root node/);
  });

  it("<Slide master> 와 자식 master 가 충돌하면 에러가 난다", () => {
    expect(() =>
      parseBuilderDocument(`
        <SlideGlance>
          <Slide master="A">
            <VStack master="B"><Text>X</Text></VStack>
          </Slide>
        </SlideGlance>
      `),
    ).toThrow(/Conflicting master assignment/);
  });

  it("Presentation 안의 Styles/class 를 슬라이드와 Master 양쪽에 적용할 수 있다", () => {
    const { document } = parseBuilderDocument(`
      <SlideGlance><Document defaultMaster="PRIMARY" />
        <Styles>
          <Style name="page" padding="48" backgroundColor="F8FAFC" />
          <Style name="title" fontSize="24" bold="true" color="1E293B" />
        </Styles>
        <Master name="PRIMARY">
          <VStack class="page">
            <Text class="title">Header</Text>
          </VStack>
        </Master>
        <VStack class="page">
          <Text class="title">Body</Text>
        </VStack>
      </SlideGlance>
    `);

    expect(document.masterContents?.PRIMARY?.[0]).toMatchObject({
      type: "vstack",
      padding: 48,
      backgroundColor: "F8FAFC",
      children: [
        {
          type: "text",
          text: "Header",
          fontSize: 24,
          bold: true,
          color: "1E293B",
        },
      ],
    });
    expect(document.nodes[0]).toMatchObject({
      type: "vstack",
      padding: 48,
      backgroundColor: "F8FAFC",
      children: [
        {
          type: "text",
          text: "Body",
          fontSize: 24,
          bold: true,
          color: "1E293B",
        },
      ],
    });
  });

  it("Presentation 기본 텍스트 스타일 속성이 잘못되면 에러가 난다", () => {
    expect(() =>
      parseBuilderDocument(`
        <SlideGlance><Document fontSize="abc" bold="yes" />
          <VStack><Text>Hello</Text></VStack>
        </SlideGlance>
      `),
    ).toThrow(
      /Invalid value for attribute "fontSize"|Invalid value for attribute "bold"/,
    );
  });
});
