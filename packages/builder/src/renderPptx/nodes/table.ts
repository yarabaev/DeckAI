import type { PositionedNode } from "../../types.ts";
import type { RenderContext } from "../types.ts";
import {
  resolveFontFamily,
  resolveTextStyleValue,
} from "../../defaultTextStyle.ts";
import {
  resolveColumnWidths,
  resolveRowHeights,
} from "../../shared/tableUtils.ts";
import { pxToIn, pxToPt } from "../units.ts";
import { convertUnderline, convertStrike } from "../textOptions.ts";
import { getContentArea } from "../utils/contentArea.ts";
import { renderObjectName } from "../utils/objectName.ts";
import { validateHref } from "../utils/href.ts";

type TablePositionedNode = Extract<PositionedNode, { type: "table" }>;

// Default cell margin in px. Converted to pt per pptxgenjs expectation.
// ~0.05in vertical / ~0.1in horizontal matches PowerPoint's default table margin.
const DEFAULT_CELL_MARGIN_PX: [number, number, number, number] = [5, 10, 5, 10];

type MarginInput =
  | number
  | { top?: number; right?: number; bottom?: number; left?: number }
  | undefined;

function resolveCellMarginPt(
  cellMargin: MarginInput,
  tableMargin: MarginInput,
): [number, number, number, number] {
  const [dt, dr, db, dl] = DEFAULT_CELL_MARGIN_PX;
  const pick = (
    m: MarginInput,
    fallback: [number, number, number, number],
  ): [number, number, number, number] => {
    if (m === undefined) return fallback;
    if (typeof m === "number") return [m, m, m, m];
    return [
      m.top ?? fallback[0],
      m.right ?? fallback[1],
      m.bottom ?? fallback[2],
      m.left ?? fallback[3],
    ];
  };
  const tableResolved = pick(tableMargin, [dt, dr, db, dl]);
  const cellResolved = pick(cellMargin, tableResolved);
  return [
    pxToPt(cellResolved[0]),
    pxToPt(cellResolved[1]),
    pxToPt(cellResolved[2]),
    pxToPt(cellResolved[3]),
  ];
}

export function renderTableNode(
  node: TablePositionedNode,
  ctx: RenderContext,
): void {
  const defaultTextStyle = ctx.buildContext?.defaultTextStyle;
  const tableRows = node.rows.map((row) =>
    row.cells.map((cell) => {
      const cellMarginPt = resolveCellMarginPt(
        cell.padding ?? cell.margin,
        node.cellMargin,
      );
      const cellOptions: Record<string, unknown> = {
        fontSize: pxToPt(
          resolveTextStyleValue(cell.fontSize, defaultTextStyle?.fontSize, 18),
        ),
        fontFace: resolveFontFamily(cell.fontFamily, defaultTextStyle),
        color: cell.color ?? defaultTextStyle?.color,
        bold: cell.bold ?? defaultTextStyle?.bold,
        italic: cell.italic ?? defaultTextStyle?.italic,
        underline: convertUnderline(cell.underline),
        strike: convertStrike(cell.strike),
        highlight: cell.highlight,
        align: cell.textAlign ?? "left",
        valign: cell.verticalAlign ?? "middle",
        fill: cell.backgroundColor
          ? { color: cell.backgroundColor }
          : undefined,
        colspan: cell.colspan,
        rowspan: cell.rowspan,
        margin: cellMarginPt,
        charSpacing:
          cell.letterSpacing !== undefined
            ? cell.letterSpacing * 100
            : undefined,
      };

      if (cell.runs && cell.runs.length > 0) {
        const textItems = cell.runs.map((run) => {
          const validatedHref = run.href
            ? validateHref(
                run.href,
                ctx.buildContext.security.allowedHrefSchemes,
                ctx,
              )
            : undefined;
          return {
            text: run.text,
            options: {
              fontSize: pxToPt(
                resolveTextStyleValue(
                  cell.fontSize,
                  defaultTextStyle?.fontSize,
                  18,
                ),
              ),
              fontFace: resolveFontFamily(cell.fontFamily, defaultTextStyle),
              color: run.color ?? cell.color ?? defaultTextStyle?.color,
              bold: run.bold ?? cell.bold ?? defaultTextStyle?.bold,
              italic: run.italic ?? cell.italic ?? defaultTextStyle?.italic,
              underline: convertUnderline(run.underline ?? cell.underline),
              strike: convertStrike(run.strike ?? cell.strike),
              highlight: run.highlight ?? cell.highlight,
              charSpacing:
                cell.letterSpacing !== undefined
                  ? cell.letterSpacing * 100
                  : undefined,
              ...(validatedHref ? { hyperlink: { url: validatedHref } } : {}),
            },
          };
        });
        return {
          text: textItems,
          options: {
            align: cell.textAlign ?? "left",
            valign: cell.verticalAlign ?? "middle",
            fill: cell.backgroundColor
              ? { color: cell.backgroundColor }
              : undefined,
            colspan: cell.colspan,
            rowspan: cell.rowspan,
            margin: cellMarginPt,
            charSpacing:
              cell.letterSpacing !== undefined
                ? cell.letterSpacing * 100
                : undefined,
          },
        };
      }

      return {
        text: cell.text,
        options: cellOptions,
      };
    }),
  );

  const content = getContentArea(node);
  const objectName = renderObjectName(node, ctx);
  const tableOptions: Record<string, unknown> = {
    x: pxToIn(content.x),
    y: pxToIn(content.y),
    w: pxToIn(content.w),
    h: pxToIn(content.h),
    colW: resolveColumnWidths(node, content.w).map((width) => pxToIn(width)),
    rowH: resolveRowHeights(node).map((height) => pxToIn(height)),
    valign: "middle",
    ...(objectName ? { objectName } : {}),
  };

  if (node.cellBorder) {
    tableOptions.border = {
      color: node.cellBorder.color ?? "000000",
      pt:
        node.cellBorder.width !== undefined ? pxToPt(node.cellBorder.width) : 1,
      type: node.cellBorder.dashType ?? "solid",
    };
  }

  ctx.slide.addTable(tableRows, tableOptions);
}
