import { autoFitSlide } from "./autoFit/autoFit.ts";
import { createBuildContext } from "./buildContext.ts";
import { calcYogaLayout } from "./calcYogaLayout/calcYogaLayout.ts";
import { lintDeck, type LintOptions, type LintReport } from "./lint/index.ts";
import type { TextMeasurementMode } from "./calcYogaLayout/measureText.ts";
import type { YogaNodeMap } from "./calcYogaLayout/types.ts";
import { extractLayoutResults } from "./calcYogaLayout/types.ts";
import type { Diagnostic } from "./diagnostics.ts";
import { DiagnosticsError } from "./diagnostics.ts";
import { parseMasterPptx } from "./parseMasterPptx.ts";
import {
  parseBuilderDocument,
  type ImportResolver,
  type BuilderSourceMap,
} from "./parseXml/parseXml.ts";
import { validateImageSrc } from "./renderPptx/nodes/image.ts";
import { renderPptx } from "./renderPptx/renderPptx.ts";
import { wrapPptxWriteWithConnectors } from "./renderPptx/postProcess/wrapWrite.ts";
import { freeYogaTree } from "./shared/freeYogaTree.ts";
import { toPositioned } from "./toPositioned/toPositioned.ts";
import { mergeDefaultTextStyles } from "./defaultTextStyle.ts";
import {
  BuilderNode,
  PositionedNode,
  SlideMasterOptions,
  type DefaultTextStyle,
} from "./types.ts";

export type { TextMeasurementMode };

export interface BuildPptxResult {
  pptx: import("pptxgenjs").default;
  diagnostics: Diagnostic[];
  /**
   * When `options.trackSourcePos` is true, maps BuilderNode.__nodeId → origin
   * { file, line } so that consumers (e.g. the builder-vscode webview) can reveal
   * the source file/line for rendered pptxgenjs objects.
   */
  sourceMap?: BuilderSourceMap;
  /**
   * Structured lint report when `options.lint?.enabled` is true.
   * Same diagnostics that get merged into `diagnostics`, plus per-deck
   * summary counts and a generatedAt stamp suitable for tooling /
   * LLM consumption.
   */
  lintReport?: LintReport;
}

export type { ImageSrcGuardOptions, MasterPptxLimits } from "./options.ts";
import type { ImageSrcGuardOptions, MasterPptxLimits } from "./options.ts";

export interface BuildPptxOptions {
  master?: SlideMasterOptions;
  masters?: SlideMasterOptions[];
  defaultMaster?: string;
  masterPptx?: ArrayBuffer | Uint8Array;
  textMeasurement?: TextMeasurementMode;
  defaultTextStyle?: DefaultTextStyle;
  autoFit?: boolean;
  strict?: boolean;
  /** Resolver for `<Import src="..."/>`. The resolver is called synchronously
   *  and must return both the file content and its absolute path. */
  resolveImport?: ImportResolver;
  /** Absolute path of the root document. Passed as `fromPath` to the first
   *  `resolveImport` call so relative paths resolve correctly. */
  sourcePath?: string;
  /**
   * If true, the returned result includes `sourceMap` and every rendered
   * shape carries a pptxgenjs `objectName` encoding its `__nodeId`. Used by
   * tools (e.g. the builder-vscode preview) to offer jump-to-source on clicks.
   */
  trackSourcePos?: boolean;
  /**
   * Additional URL schemes to allow on top of the defaults (https/http/mailto/tel).
   * The defaults are always enforced. Supply only the extra schemes to permit
   * (e.g. `["ftp:"]`). Schemes not in the combined list emit `INVALID_HREF_SCHEME`.
   */
  allowedHrefSchemes?: string[];
  /**
   * Opt-in guard for <Image src> and <Master backgroundPath> validation.
   * When undefined (default), no validation is applied (OD3 locked).
   */
  imageSrcGuard?: ImageSrcGuardOptions;
  /**
   * Size caps for the masterPptx buffer. Defaults: 50 MB total, 5 MB per image.
   */
  masterPptxLimits?: MasterPptxLimits;
  /**
   * Maximum number of nodes produced by template expansion. Default: 100,000.
   * Exceeding this limit emits a TEMPLATE_EXPANSION_LIMIT diagnostic.
   */
  maxTemplateNodes?: number;
  /**
   * When true, run the equalize-dimensions preprocessor before parsing —
   * resolves `auto`, `auto:KEY`, and `capbar:CLASS` sentinels by measuring
   * sibling content and substituting concrete pixel values. See
   * `ParseBuilderDocumentOptions.equalize` for details.
   */
  equalize?: boolean;
  /** Document properties written to the PPTX file's core properties (docProps/core.xml). */
  docProps?: {
    title?: string;
    author?: string;
    company?: string;
    subject?: string;
  };
  /**
   * Default BCP 47 language tag applied to text runs that do not carry an
   * explicit `lang` attribute (e.g. `"en-US"`, `"ja-JP"`). Default: undefined.
   */
  defaultLang?: string;
  /**
   * Lint options. When `lint.enabled` is true, post-layout lint rules
   * run after every slide is positioned and the resulting Diagnostics
   * are merged into the BuildPptxResult.diagnostics list. The full
   * structured `LintReport` is also returned for tooling / LLM input.
   * See `packages/builder/docs/lint.md` for the rule catalog.
   */
  lint?: LintOptions;
  /**
   * TTF / OTF font buffers to register with the per-build text
   * measurer alongside the bundled fonts (Noto Sans JP + Pretendard,
   * Regular + Bold). When present, opentype measurement looks up the
   * caller's families directly (no Noto/Pretendard substitution), so
   * the wrap decision the builder commits to the PPTX matches the
   * glyph metrics of the actual font the renderer will paint.
   *
   * Supply both weight variants (Regular and Bold) of any family that
   * appears in `bold="true"` text — slideglance routes faces with
   * `OS/2.usWeightClass >= 600` to the resolver's bold slot, so a
   * single family lookup with `bold=true` reaches the Bold face.
   *
   * Pair with the viewer's `fontStylesheet` prop using the same TTFs
   * (as `@font-face` data URIs) so layout-time wrap = render-time wrap.
   */
  fonts?: Uint8Array[];
}

const DEFAULT_MASTER_NAME = "SLIDEGLANCE_MASTER";

function normalizeMasters(
  masters: SlideMasterOptions[] | undefined,
): SlideMasterOptions[] | undefined {
  if (!masters || masters.length === 0) return undefined;

  return masters.map((master, index) => ({
    ...master,
    title:
      master.title ??
      (masters.length === 1
        ? DEFAULT_MASTER_NAME
        : `${DEFAULT_MASTER_NAME}_${index + 1}`),
  }));
}

async function positionNode(
  node: BuilderNode,
  slideSize: { w: number; h: number },
  autoFitEnabled: boolean,
  ctx: ReturnType<typeof createBuildContext>,
): Promise<PositionedNode> {
  let map: YogaNodeMap | undefined;
  try {
    if (autoFitEnabled) {
      map = await autoFitSlide(node, slideSize, ctx);
    } else {
      map = await calcYogaLayout(node, slideSize, ctx);
    }
    const layoutMap = extractLayoutResults(map);
    return await toPositioned(node, ctx, layoutMap);
  } finally {
    if (map) freeYogaTree(map);
  }
}

function mergeMasterBackground(
  masters: SlideMasterOptions[] | undefined,
  availableMasterNames: string[],
  defaultMaster: string | undefined,
  background: NonNullable<SlideMasterOptions["background"]>,
): SlideMasterOptions[] {
  if (!masters || masters.length === 0) {
    return [
      {
        title: defaultMaster ?? availableMasterNames[0] ?? DEFAULT_MASTER_NAME,
        background,
      },
    ];
  }

  const targetTitle = defaultMaster ?? masters[0]?.title;
  if (!targetTitle) return masters;

  return masters.map((master) =>
    master.title === targetTitle && !master.background
      ? { ...master, background }
      : master,
  );
}

/**
 * Validates <Master backgroundPath> entries against the imageSrcGuard policy.
 * Clears disallowed background paths (emits INVALID_IMAGE_SRC diagnostics).
 */
function applyImageSrcGuardToMasters(
  masters: SlideMasterOptions[] | undefined,
  guard: ImageSrcGuardOptions,
  diagnostics: import("./diagnostics.ts").DiagnosticCollector,
): SlideMasterOptions[] | undefined {
  if (!masters) return masters;
  return masters.map((master) => {
    const bg = master.background;
    if (bg && "path" in bg && typeof bg.path === "string") {
      const validated = validateImageSrc(bg.path, guard, diagnostics);
      if (validated === undefined) {
        return { ...master, background: undefined };
      }
    }
    return master;
  });
}

function validateMasterReferences(
  pages: PositionedNode[],
  masters: SlideMasterOptions[] | undefined,
  masterContents: Record<string, PositionedNode[]> | undefined,
  defaultMaster: string | undefined,
): void {
  const duplicateTitles = new Set<string>();
  const titleSet = new Set<string>();
  const masterTitles = (masters ?? [])
    .map((master) => master.title)
    .filter((title): title is string => typeof title === "string");

  for (const title of masterTitles) {
    if (titleSet.has(title)) duplicateTitles.add(title);
    titleSet.add(title);
  }

  for (const title of Object.keys(masterContents ?? {})) {
    titleSet.add(title);
  }

  if (titleSet.size === 0) {
    const refs = [
      ...(defaultMaster ? [defaultMaster] : []),
      ...pages
        .map((page) => page.master)
        .filter((name): name is string => typeof name === "string"),
    ];
    if (refs.length > 0) {
      throw new Error(
        `Unknown slide master reference: "${refs[0]}". No masters are defined.`,
      );
    }
    return;
  }

  for (const title of duplicateTitles) {
    throw new Error(`Duplicate slide master title: "${title}"`);
  }

  if (defaultMaster && !titleSet.has(defaultMaster)) {
    throw new Error(`Unknown default slide master: "${defaultMaster}"`);
  }

  for (const page of pages) {
    if (page.master && !titleSet.has(page.master)) {
      throw new Error(`Unknown slide master reference: "${page.master}"`);
    }
  }
}

export async function buildPptx(
  xml: string,
  slideSize: { w: number; h: number },
  options?: BuildPptxOptions,
): Promise<BuildPptxResult> {
  // Capture the root document plus every successfully resolved import
  // so the parse-phase lint pass (RAW_LT_GT_IN_ATTR etc.) sees the
  // original characters before fast-xml-parser quietly normalises
  // them. Only collect when lint is actually enabled — the resolver
  // wrap is otherwise an unnecessary allocation per import call.
  const lintEnabled = options?.lint?.enabled === true;
  const rawXmlSources: { path?: string; content: string }[] = lintEnabled
    ? [
        {
          content: xml,
          ...(options?.sourcePath ? { path: options.sourcePath } : {}),
        },
      ]
    : [];
  const originalResolveImport = options?.resolveImport;
  const wrappedResolveImport: typeof originalResolveImport =
    lintEnabled && originalResolveImport
      ? (src, fromPath) => {
          const resolved = originalResolveImport(src, fromPath);
          rawXmlSources.push({
            content: resolved.content,
            path: resolved.path,
          });
          return resolved;
        }
      : originalResolveImport;

  const parseResult = parseBuilderDocument(xml, {
    resolveImport: wrappedResolveImport,
    sourcePath: options?.sourcePath,
    trackSourcePos: options?.trackSourcePos,
    maxTemplateNodes: options?.maxTemplateNodes,
    equalize: options?.equalize,
  });
  const document = parseResult.document;
  const ctx = createBuildContext({
    textMeasurementMode: options?.textMeasurement ?? "auto",
    defaultTextStyle: mergeDefaultTextStyles(
      document.defaultTextStyle,
      options?.defaultTextStyle,
    ),
    allowedHrefSchemes: options?.allowedHrefSchemes,
    imageSrcGuard: options?.imageSrcGuard,
    defaultLang: options?.defaultLang,
    fonts: options?.fonts,
  });
  // T11.5: surface non-fatal parse-time diagnostics through the same
  // collector that gathers render-time diagnostics, so the final
  // BuildPptxResult exposes them via .diagnostics in arrival order.
  ctx.diagnostics.addAll(parseResult.diagnostics);
  const resolvedSlideSize = document.slideSize ?? slideSize;
  const nodes = document.nodes;
  const baseMasters =
    options?.masters ??
    document.masters ??
    (options?.master ? [options.master] : undefined);
  let masters = normalizeMasters(baseMasters);
  // T37: apply imageSrcGuard to <Master backgroundPath> entries if guard is active
  if (options?.imageSrcGuard && masters) {
    masters = applyImageSrcGuardToMasters(
      masters,
      options.imageSrcGuard,
      ctx.diagnostics,
    );
  }
  const masterContentNodes = document.masterContents;
  let defaultMaster =
    options?.defaultMaster ??
    document.defaultMaster ??
    (masters && masters.length > 0 ? masters[0]?.title : undefined);
  const normalizedMasterTitles = (masters ?? [])
    .map((master) => master.title)
    .filter((title): title is string => typeof title === "string");

  const positionedPages: PositionedNode[] = [];
  const positionedMasterContents: Record<string, PositionedNode[]> = {};
  const availableMasterNames = [
    ...normalizedMasterTitles,
    ...Object.keys(masterContentNodes ?? {}),
  ];

  for (const node of nodes) {
    const positioned = await positionNode(
      node,
      resolvedSlideSize,
      options?.autoFit !== false,
      ctx,
    );
    positionedPages.push(positioned);
  }

  if (masterContentNodes) {
    for (const [masterName, contentNodes] of Object.entries(
      masterContentNodes,
    )) {
      positionedMasterContents[masterName] = [];
      for (const node of contentNodes) {
        const positioned = await positionNode(
          node,
          resolvedSlideSize,
          options?.autoFit !== false,
          ctx,
        );
        positionedMasterContents[masterName].push(positioned);
      }
    }
  }

  // Extract background from masterPptx and merge into master settings
  if (options?.masterPptx) {
    try {
      const bg = await parseMasterPptx(options.masterPptx, {
        limits: options.masterPptxLimits,
        diagnostics: ctx.diagnostics,
      });
      if (bg) {
        masters = mergeMasterBackground(
          masters,
          availableMasterNames,
          defaultMaster,
          bg,
        );
        defaultMaster ??= masters[0]?.title;
      }
    } catch (e) {
      const message =
        e instanceof Error ? e.message : "Unknown error parsing masterPptx";
      ctx.diagnostics.add("MASTER_PPTX_PARSE_FAILED", message);
    }
  }

  validateMasterReferences(
    positionedPages,
    masters,
    Object.keys(positionedMasterContents).length > 0
      ? positionedMasterContents
      : undefined,
    defaultMaster,
  );

  // Post-layout lint pass. Runs over every positioned slide tree and
  // merges its findings into the same diagnostic stream the renderer
  // and parser write into — so callers get one unified list.
  let lintReport: LintReport | undefined;
  if (options?.lint?.enabled) {
    const { diagnostics: lintDiags, report } = lintDeck(
      positionedPages.map((root) => ({ root, slideSize: resolvedSlideSize })),
      options.lint,
      {
        declaredStyles: document.declaredStyles,
        referencedStyles: document.referencedStyles,
        declaredTemplates: document.declaredTemplates,
        referencedTemplates: document.referencedTemplates,
        measurer: ctx.measurer,
        rawXmlSources,
      },
    );
    for (const d of lintDiags) ctx.diagnostics.addLint(d);
    lintReport = report;
  }

  const pptx = await renderPptx(
    positionedPages,
    resolvedSlideSize,
    ctx,
    masters,
    defaultMaster,
    Object.keys(positionedMasterContents).length > 0
      ? positionedMasterContents
      : undefined,
    options?.docProps,
  );

  // Install the connector post-process pass over pptx.write / pptx.writeFile.
  // The wrapped write rewrites placeholder lines into real <p:cxnSp>
  // elements with stCxn / endCxn bindings before user-visible bytes leave
  // the builder. Strict mode still snapshots diagnostics at this point —
  // any CONNECTOR_UNKNOWN_SHAPE_IDX records that surface during write
  // join the BuildPptxResult.diagnostics list but do not retro-trigger
  // strict-throw.
  wrapPptxWriteWithConnectors(pptx, ctx.diagnostics);

  const diagnostics = ctx.diagnostics.items;

  if (options?.strict && diagnostics.length > 0) {
    throw new DiagnosticsError(diagnostics);
  }

  return {
    pptx,
    diagnostics,
    ...(document.sourceMap ? { sourceMap: document.sourceMap } : {}),
    ...(lintReport ? { lintReport } : {}),
  };
}
