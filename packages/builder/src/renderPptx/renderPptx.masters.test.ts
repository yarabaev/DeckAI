import { beforeEach, describe, expect, it, vi } from "vitest";
import { createBuildContext } from "../buildContext.ts";

const defineLayout = vi.fn();
const defineSlideMaster = vi.fn();
const addSlide = vi.fn((_opts?: { masterName?: string }) => ({
  addText: vi.fn(),
  addImage: vi.fn(),
  addShape: vi.fn(),
  addTable: vi.fn(),
  addNotes: vi.fn(),
}));

class MockPptxGenJS {
  layout = "";
  defineLayout = defineLayout;
  defineSlideMaster = defineSlideMaster;
  addSlide = addSlide;
}

vi.mock("pptxgenjs", () => ({
  default: MockPptxGenJS,
}));

import { renderPptx } from "./renderPptx.ts";

describe("renderPptx masters", () => {
  beforeEach(() => {
    defineLayout.mockReset();
    defineSlideMaster.mockReset();
    addSlide.mockClear();
  });

  it("default master 와 페이지별 master 를 함께 적용한다", async () => {
    await renderPptx(
      [
        {
          type: "vstack",
          x: 0,
          y: 0,
          w: 1280,
          h: 720,
          children: [],
        },
        {
          type: "vstack",
          x: 0,
          y: 0,
          w: 1280,
          h: 720,
          master: "ALT",
          children: [],
        },
      ] as never,
      { w: 1280, h: 720 },
      createBuildContext({ textMeasurementMode: "fallback" }),
      [
        { title: "PRIMARY", background: { color: "FFFFFF" } },
        {
          title: "ALT",
          objects: [
            {
              type: "rect",
              x: 0,
              y: 0,
              w: 1280,
              h: 40,
              fill: { color: "0F172A" },
            },
          ],
        },
      ],
      "PRIMARY",
    );

    expect(defineSlideMaster).toHaveBeenCalledTimes(2);
    expect(defineSlideMaster.mock.calls[0]?.[0]).toMatchObject({
      title: "PRIMARY",
    });
    expect(defineSlideMaster.mock.calls[1]?.[0]).toMatchObject({
      title: "ALT",
    });
    expect(addSlide.mock.calls[0]?.[0]).toEqual({ masterName: "PRIMARY" });
    expect(addSlide.mock.calls[1]?.[0]).toEqual({ masterName: "ALT" });
  });

  it("slide notes 를 speaker notes 로 기록한다", async () => {
    await renderPptx(
      [
        {
          type: "vstack",
          x: 0,
          y: 0,
          w: 1280,
          h: 720,
          notes: "Presenter note",
          children: [],
        },
      ] as never,
      { w: 1280, h: 720 },
      createBuildContext({ textMeasurementMode: "fallback" }),
    );

    expect(addSlide).toHaveBeenCalledTimes(1);
    const slide = addSlide.mock.results[0]?.value as {
      addNotes: ReturnType<typeof vi.fn>;
    };
    expect(slide.addNotes).toHaveBeenCalledWith("Presenter note");
  });

  it("Master content 를 true master object 로 변환한다", async () => {
    await renderPptx(
      [
        {
          type: "vstack",
          x: 0,
          y: 0,
          w: 1280,
          h: 720,
          children: [],
        },
      ] as never,
      { w: 1280, h: 720 },
      createBuildContext({ textMeasurementMode: "fallback" }),
      [{ title: "PRIMARY" }],
      "PRIMARY",
      {
        PRIMARY: [
          {
            type: "vstack",
            x: 0,
            y: 0,
            w: 1280,
            h: 720,
            backgroundColor: "F8FAFC",
            children: [
              {
                type: "text",
                text: "Header",
                x: 48,
                y: 12,
                w: 200,
                h: 28,
                fontSize: 14,
                color: "111827",
              },
              {
                type: "shape",
                shapeType: "roundRect",
                x: 48,
                y: 64,
                w: 160,
                h: 40,
                text: "CTA",
                fill: { color: "1D4ED8" },
                color: "FFFFFF",
              },
            ],
          },
        ],
      },
    );

    expect(defineSlideMaster).toHaveBeenCalledTimes(1);
    expect(defineSlideMaster.mock.calls[0]?.[0]).toMatchObject({
      title: "PRIMARY",
      background: { color: "F8FAFC" },
    });
    expect(defineSlideMaster.mock.calls[0]?.[0].objects).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          text: expect.objectContaining({
            text: "Header",
          }),
        }),
        expect.objectContaining({
          text: expect.objectContaining({
            text: "CTA",
            options: expect.objectContaining({ shape: "roundRect" }),
          }),
        }),
      ]),
    );
  });

  it("문서 기본 텍스트 스타일을 slide 와 master 에 적용한다", async () => {
    await renderPptx(
      [
        {
          type: "vstack",
          x: 0,
          y: 0,
          w: 1280,
          h: 720,
          children: [
            {
              type: "text",
              text: "Body",
              x: 48,
              y: 48,
              w: 200,
              h: 40,
            },
          ],
        },
      ] as never,
      { w: 1280, h: 720 },
      createBuildContext({
        textMeasurementMode: "fallback",
        defaultTextStyle: {
          fontFamily: "Pretendard",
          fontSize: 20,
          color: "334155",
          bold: true,
        },
      }),
      [
        {
          title: "PRIMARY",
          objects: [
            {
              type: "text",
              text: "Header",
              x: 40,
              y: 12,
              w: 200,
              h: 24,
            },
          ],
          slideNumber: {
            x: 1200,
            y: 680,
          },
        },
      ],
      "PRIMARY",
    );

    expect(defineSlideMaster.mock.calls[0]?.[0]).toMatchObject({
      objects: expect.arrayContaining([
        expect.objectContaining({
          text: expect.objectContaining({
            text: "Header",
            options: expect.objectContaining({
              fontFace: "Pretendard",
              fontSize: 15,
              color: "334155",
              bold: true,
            }),
          }),
        }),
      ]),
      slideNumber: expect.objectContaining({
        fontFace: "Pretendard",
        fontSize: 15,
        color: "334155",
      }),
    });

    const slide = addSlide.mock.results[0]?.value as {
      addText: ReturnType<typeof vi.fn>;
    };
    expect(slide.addText).toHaveBeenCalledWith(
      "Body",
      expect.objectContaining({
        fontFace: "Pretendard",
        fontSize: 15,
        color: "334155",
        bold: true,
      }),
    );
  });

  it("Master content 에서 Icon 과 Svg 도 master object 로 변환한다", async () => {
    await renderPptx(
      [
        {
          type: "vstack",
          x: 0,
          y: 0,
          w: 1280,
          h: 720,
          children: [],
        },
      ] as never,
      { w: 1280, h: 720 },
      createBuildContext({ textMeasurementMode: "fallback" }),
      [{ title: "PRIMARY" }],
      "PRIMARY",
      {
        PRIMARY: [
          {
            type: "layer",
            x: 0,
            y: 0,
            w: 1280,
            h: 720,
            children: [
              {
                type: "icon",
                name: "star",
                x: 48,
                y: 32,
                w: 32,
                h: 32,
                variant: "circle-filled",
                bgColor: "1D4ED8",
                iconImageData: "data:image/png;base64,ICON",
                bgX: 40,
                bgY: 24,
                bgW: 48,
                bgH: 48,
                iconX: 48,
                iconY: 32,
                iconW: 32,
                iconH: 32,
              },
              {
                type: "svg",
                svgContent: "<svg/>",
                x: 120,
                y: 24,
                w: 64,
                h: 64,
                iconImageData: "data:image/png;base64,SVG",
              },
            ],
          },
        ],
      },
    );

    expect(defineSlideMaster).toHaveBeenCalledTimes(1);
    expect(defineSlideMaster.mock.calls[0]?.[0].objects).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          text: expect.objectContaining({
            text: "",
            options: expect.objectContaining({ shape: "ellipse" }),
          }),
        }),
        expect.objectContaining({
          image: expect.objectContaining({
            data: "data:image/png;base64,ICON",
          }),
        }),
        expect.objectContaining({
          image: expect.objectContaining({
            data: "data:image/png;base64,SVG",
          }),
        }),
      ]),
    );
  });

  it("Master content 의 Table 은 true master 로 변환하지 못하면 에러를 던진다", async () => {
    await expect(
      renderPptx(
        [
          {
            type: "vstack",
            x: 0,
            y: 0,
            w: 1280,
            h: 720,
            children: [],
          },
        ] as never,
        { w: 1280, h: 720 },
        createBuildContext({ textMeasurementMode: "fallback" }),
        [{ title: "PRIMARY" }],
        "PRIMARY",
        {
          PRIMARY: [
            {
              type: "table",
              x: 0,
              y: 0,
              w: 200,
              h: 64,
              columns: [{ width: 100 }, { width: 100 }],
              rows: [{ cells: [{ text: "A" }, { text: "B" }] }],
            },
          ],
        },
      ),
    ).rejects.toThrow("Table nodes are not supported in true slide masters");
  });
});
