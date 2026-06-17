export { buildPptx } from "./buildPptx.ts";
export type {
  BuildPptxResult,
  BuildPptxOptions,
  TextMeasurementMode,
  ImageSrcGuardOptions,
  MasterPptxLimits,
} from "./buildPptx.ts";
export { DiagnosticsError } from "./diagnostics.ts";
export type { Diagnostic, DiagnosticCode } from "./diagnostics.ts";
export { ParseXmlError, parseBuilderDocument } from "./parseXml/parseXml.ts";
export {
  equalizeAll,
  collectStylesMap,
  type StyleEntry,
} from "./preprocess/equalize.ts";
export type {
  ParseResult,
  ParsedBuilderDocument,
  ParseBuilderDocumentOptions,
  ImportResolver,
  BuilderSourceMap,
  BuilderSourcePos,
} from "./parseXml/parseXml.ts";
export type {
  DefaultTextStyle,
  SlideMasterOptions,
  SlideMasterBackground,
  SlideMasterMargin,
  MasterObject,
  MasterTextObject,
  MasterImageObject,
  MasterRectObject,
  MasterLineObject,
  SlideNumberOptions,
  // BuilderNode unions
  BuilderNode,
  PositionedNode,
  PositionedLayerChild,
  // Node types
  TextNode,
  UlNode,
  OlNode,
  LiNode,
  ImageNode,
  IconNode,
  SvgNode,
  TableNode,
  ShapeNode,
  ChartNode,
  LineNode,
  LineArrow,
  VStackNode,
  HStackNode,
  LayerNode,
  // Style atoms (re-exported from registry/shared via types.ts)
  Length,
  Padding,
  BorderDash,
  BorderStyle,
  FillStyle,
  ShadowStyle,
  Underline,
  UnderlineStyle,
  AlignItems,
  AlignSelf,
  PositionType,
  FlexWrap,
  JustifyContent,
  BulletNumberType,
  ShapeType,
  BackgroundImage,
  BackgroundImageSizing,
} from "./types.ts";
