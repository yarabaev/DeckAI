/**
 * Localization for the viewer UI.
 *
 * The viewer ships a small message catalog covering every
 * user-visible string in the shell, dialogs, status bar, and UI
 * components. The active locale is decided in this priority order:
 *
 *   1. Forced override via {@link setLocale} / settings.locale (any
 *      value other than `"auto"`).
 *   2. Environment auto-detect — `navigator.languages` in the
 *      browser, `LC_ALL` / `LC_MESSAGES` / `LANG` on Node (used by
 *      Vitest / SSR scenarios).
 *   3. English as the universal fallback.
 *
 * Missing keys in a non-English catalog fall back to the English
 * entry so a partially-translated catalog never surfaces empty UI.
 *
 * The module exposes a tiny pub/sub so reactive shells (Lit
 * elements) can re-render when the active locale changes without
 * pulling in a runtime dependency on a heavier i18n library — the
 * viewer is offline-first and we don't ship Intl message-format
 * polyfills.
 */

/** Forced override values plus the special `"auto"` sentinel. */
export type Locale =
  | "auto"
  | "en"
  | "ko"
  | "ja"
  | "zh-CN"
  | "zh-TW"
  | "es"
  | "fr"
  | "de";

/** A concrete locale — what {@link getActiveLocale} returns. */
export type ResolvedLocale = Exclude<Locale, "auto">;

/** All concrete locales the viewer ships catalogs for. */
export const SUPPORTED_LOCALES: readonly ResolvedLocale[] = Object.freeze([
  "en",
  "ko",
  "ja",
  "zh-CN",
  "zh-TW",
  "es",
  "fr",
  "de",
]);

/**
 * Every translatable string used by the viewer, keyed by a stable
 * dotted identifier. Keep the keys language-neutral so translators
 * see semantics rather than English source text — this also keeps
 * the call sites grep-able.
 *
 * Tokens use `{name}` syntax and are substituted by {@link t}.
 */
export type MessageKey =
  | "common.close"
  | "common.cancel"
  | "common.loading"
  | "common.ready"
  | "common.bytes"
  // Ribbon — navigation
  | "nav.firstSlide"
  | "nav.previousSlide"
  | "nav.nextSlide"
  | "nav.lastSlide"
  | "nav.slideCounter"
  | "nav.slideCounterEmpty"
  // Ribbon — search / output / slideshow / settings
  | "search.button"
  | "search.placeholder"
  | "search.title"
  | "search.empty"
  | "search.noMatches"
  | "search.typeToSearch"
  | "output.print"
  | "output.printTitle"
  | "output.pdf"
  | "output.pdfTitle"
  | "output.slideshow"
  | "output.slideshowTitle"
  | "output.gateLoadFirst"
  | "output.gatePreparing"
  | "settings.title"
  | "settings.openTitle"
  // File operations (ribbon)
  | "file.open"
  // Render mode selector — controls SVG text rendering mode
  // (vector `<text>` vs path outline), NOT user text selection.
  | "render.label"
  | "render.text"
  | "render.path"
  | "render.auto"
  // Status bar
  | "status.toggleNotes"
  | "status.resizeSidebar"
  | "status.normalView"
  | "status.gridView"
  | "status.zoomOut"
  | "status.zoomIn"
  | "status.zoomReset"
  | "status.fitWindow"
  | "status.zoom"
  | "status.slideOf"
  | "status.slideEmpty"
  | "status.selectionFontLabel"
  | "status.selectionFontMultiple"
  | "status.selectionFontTitle"
  // Font usage indicator (status bar)
  | "fontUsage.title"
  | "fontUsage.close"
  | "fontUsage.headerRequested"
  | "fontUsage.headerEffective"
  | "fontUsage.systemFallback"
  | "fontUsage.allMatched"
  | "fontUsage.substituteCount"
  // Notes panel (presentation shell + standalone)
  | "notes.heading"
  | "notes.headingWithSection"
  | "notes.empty"
  | "notes.standaloneHeading"
  | "notes.standaloneEmpty"
  | "notes.layoutLabel"
  | "notes.sectionLabel"
  | "notes.noSlide"
  // Section nav
  | "section.empty"
  // Settings dialog
  | "dialog.title"
  | "dialog.appearance"
  | "dialog.theme"
  | "dialog.themeDesc"
  | "dialog.themeAuto"
  | "dialog.themeAutoDesc"
  | "dialog.themeLight"
  | "dialog.themeLightDesc"
  | "dialog.themeDark"
  | "dialog.themeDarkDesc"
  | "dialog.themeHighContrast"
  | "dialog.themeHighContrastDesc"
  | "dialog.ruler"
  | "dialog.rulerShow"
  | "dialog.rulerUnitLabel"
  | "dialog.rulerUnitDesc"
  | "dialog.rulerUnitCm"
  | "dialog.rulerUnitCmDesc"
  | "dialog.rulerUnitPx"
  | "dialog.rulerUnitPxDesc"
  | "dialog.language"
  | "dialog.languageDesc"
  | "dialog.languageAuto"
  | "dialog.about"
  | "dialog.aboutPackage"
  | "dialog.aboutVersion"
  | "dialog.aboutRendering"
  | "dialog.aboutRenderingValue"
  | "dialog.aboutLicense"
  | "dialog.aboutAppName"
  | "dialog.aboutEngine"
  | "dialog.aboutNpmPackage"
  | "dialog.aboutRepository"
  | "dialog.aboutCopyright"
  | "dialog.aboutDeveloper"
  | "dialog.viewerSettingsAriaLabel"
  // Status messages (dynamic phases / errors)
  | "phase.preparingSlide"
  | "phase.renderingPdf"
  | "phase.encodingPdf"
  | "phase.preparingSlides"
  | "status.loadedSlides"
  | "status.nothingToPrint"
  | "status.nothingToExport"
  | "status.pdfFailed"
  | "status.exported"
  | "status.errorPrefix"
  // Viewer overlay
  | "viewer.ariaLabel"
  | "viewer.loading"
  | "viewer.error"
  | "viewer.noFile"
  | "viewer.slideTitle"
  | "viewer.empty"
  | "viewer.openFile"
  | "phase.preparingSlideOf"
  | "phase.preparingPdf"
  | "phase.preparingPrint"
  | "phase.layingOutPrintOf"
  | "phase.openingPrintDialog"
  | "phase.savingPdf"
  | "progress.titlePrint"
  | "progress.titlePdf"
  // Playground
  | "playground.title"
  | "playground.upload"
  | "playground.samples"
  | "playground.pickPrompt"
  | "playground.loadingFile"
  | "playground.failedHttp"
  | "playground.fileInfo"
  | "playground.convertFailed"
  // Shortcuts dialog
  | "shortcuts.title"
  | "shortcuts.openTitle"
  | "shortcuts.groupNavigation"
  | "shortcuts.groupView"
  | "shortcuts.groupSelection"
  | "shortcuts.groupOutput"
  | "shortcuts.prevSlide"
  | "shortcuts.nextSlide"
  | "shortcuts.firstSlide"
  | "shortcuts.lastSlide"
  | "shortcuts.zoomIn"
  | "shortcuts.zoomOut"
  | "shortcuts.zoomReset"
  | "shortcuts.panSlide"
  | "shortcuts.click"
  | "shortcuts.drag"
  | "shortcuts.doubleClick"
  | "shortcuts.selectShape"
  | "shortcuts.toggleSelect"
  | "shortcuts.rubberBand"
  | "shortcuts.selectAll"
  | "shortcuts.copyText"
  | "shortcuts.editText"
  | "shortcuts.clearSelection"
  | "shortcuts.toggleSearch"
  | "shortcuts.print";

type Catalog = Partial<Record<MessageKey, string>> &
  Record<"_locale", ResolvedLocale>;

const EN: Catalog = {
  _locale: "en",
  "common.close": "Close",
  "common.cancel": "Cancel",
  "common.loading": "Loading…",
  "common.ready": "Ready.",
  "common.bytes": "{count} bytes",
  "nav.firstSlide": "First slide",
  "nav.previousSlide": "Previous (←)",
  "nav.nextSlide": "Next (→)",
  "nav.lastSlide": "Last slide",
  "nav.slideCounter": "{current} / {total}",
  "nav.slideCounterEmpty": "—",
  "search.button": "Search",
  "search.placeholder": "Find in slides…",
  "search.title": "Search (Cmd/Ctrl+F)",
  "search.empty": "Search",
  "search.noMatches": "No matches.",
  "search.typeToSearch": "Type to search.",
  "output.print": "Print",
  "output.printTitle": "Print (Cmd/Ctrl+P)",
  "output.pdf": "PDF",
  "output.pdfTitle": "Export PDF",
  "output.slideshow": "Slideshow",
  "output.slideshowTitle": "Start slideshow (F5)",
  "output.gateLoadFirst": "Load a deck first",
  "output.gatePreparing": "Preparing slides — {current} / {total} ready",
  "settings.title": "Settings",
  "settings.openTitle": "Settings",
  "file.open": "Open",
  "render.label": "Text Rendering:",
  "render.text": "Vector Text",
  "render.path": "Path Outline",
  "render.auto": "Auto",
  "status.toggleNotes": "Toggle notes pane",
  "status.resizeSidebar": "Drag to resize the thumbnail sidebar",
  "status.normalView": "Normal view (slide + thumbnail panel)",
  "status.gridView": "Slide sorter — show every slide at once",
  "status.zoomOut": "Zoom out",
  "status.zoomIn": "Zoom in",
  "status.zoomReset": "Reset to 100%",
  "status.fitWindow": "Fit slide to window",
  "status.zoom": "Zoom",
  "status.slideOf": "slide {current} / {total}",
  "status.slideEmpty": "—",
  "status.selectionFontLabel": "Font:",
  "status.selectionFontMultiple": "{first} +{extra} more",
  "status.selectionFontTitle": "Font(s) used by the selection: {fonts}",
  "fontUsage.title": "Font mapping",
  "fontUsage.close": "Close",
  "fontUsage.headerRequested": "Authored",
  "fontUsage.headerEffective": "Rendered as",
  "fontUsage.systemFallback": "system fallback",
  "fontUsage.allMatched": "All authored fonts available",
  "fontUsage.substituteCount": "{count} font(s) substituted",
  "notes.heading": "Notes — slide {current}",
  "notes.headingWithSection": "Notes — slide {current} · {section}",
  "notes.empty": "No notes for this slide.",
  "notes.standaloneHeading": "Notes — slide {current}",
  "notes.standaloneEmpty": "No speaker notes for this slide.",
  "notes.layoutLabel": "layout: {value}",
  "notes.sectionLabel": "section: {value}",
  "notes.noSlide": "No slide loaded.",
  "section.empty": "no sections",
  "dialog.title": "Settings",
  "dialog.appearance": "Appearance",
  "dialog.theme": "Theme",
  "dialog.themeDesc":
    "Applied to the entire viewer including the slide stage, ribbon, sidebar, and dialogs.",
  "dialog.themeAuto": "Auto",
  "dialog.themeAutoDesc": "Follow OS / browser",
  "dialog.themeLight": "Light",
  "dialog.themeLightDesc": "Always light",
  "dialog.themeDark": "Dark",
  "dialog.themeDarkDesc": "Always dark",
  "dialog.themeHighContrast": "High contrast",
  "dialog.themeHighContrastDesc": "Accessibility",
  "dialog.ruler": "Ruler",
  "dialog.rulerShow": "Show ruler around the slide",
  "dialog.rulerUnitLabel": "Unit",
  "dialog.rulerUnitDesc":
    "PowerPoint default centres the slide on 0 with cm tick marks. Pixel mode shows on-screen pixel coordinates starting at the slide edge.",
  "dialog.rulerUnitCm": "Centimetres (cm)",
  "dialog.rulerUnitCmDesc": "PowerPoint default",
  "dialog.rulerUnitPx": "Pixels (px)",
  "dialog.rulerUnitPxDesc": "On-screen pixels",
  "dialog.language": "Language",
  "dialog.languageDesc":
    "Choose the viewer interface language. Auto follows your browser / OS preference.",
  "dialog.languageAuto": "Auto ({detected})",
  "dialog.about": "About",
  "dialog.aboutAppName": "App",
  "dialog.aboutVersion": "Version",
  "dialog.aboutRendering": "Environment",
  "dialog.aboutRenderingValue": "WebAssembly · browser-only · offline-capable",
  "dialog.aboutEngine": "Engine",
  "dialog.aboutNpmPackage": "Frontend (npm)",
  "dialog.aboutPackage": "Package",
  "dialog.aboutLicense": "License",
  "dialog.aboutCopyright": "Copyright",
  "dialog.aboutDeveloper": "Developer",
  "dialog.aboutRepository": "Repository",
  "dialog.viewerSettingsAriaLabel": "Viewer settings",
  "phase.preparingSlide": "Preparing slide {current} / {total}…",
  "phase.renderingPdf": "Rendering PDF page {current} / {total}…",
  "phase.encodingPdf": "Encoding PDF…",
  "phase.preparingSlides": "Preparing slides…",
  "status.loadedSlides": "Loaded {count} slides",
  "status.nothingToPrint": "Nothing to print.",
  "status.nothingToExport": "Nothing to export.",
  "status.pdfFailed": "PDF export failed: {reason}",
  "status.exported": "Exported {count} slides to PDF.",
  "status.errorPrefix": "[error:{phase}] {message}",
  "viewer.ariaLabel": "PPTX viewer",
  "viewer.noFile": "(no file)",
  "viewer.slideTitle": "Slide {number}",
  "viewer.empty": "Open a .pptx file.",
  "viewer.openFile": "Open file",
  "phase.preparingSlideOf": "Preparing slide {current} / {total}",
  "phase.preparingPdf": "Preparing PDF…",
  "phase.preparingPrint": "Preparing print…",
  "phase.layingOutPrintOf": "Laying out page {current} / {total}…",
  "phase.openingPrintDialog": "Opening print dialog…",
  "phase.savingPdf": "Saving PDF…",
  "progress.titlePrint": "Preparing print",
  "progress.titlePdf": "Exporting PDF",
  "viewer.loading": "Loading…",
  "viewer.error": "⚠ {message}",
  "playground.title": "slideglance playground",
  "playground.upload": "upload:",
  "playground.samples": "samples:",
  "playground.pickPrompt": "Pick a sample or upload a .pptx.",
  "playground.loadingFile": "Loading {name}…",
  "playground.failedHttp": "Failed: HTTP {status}",
  "playground.fileInfo": "{name} · {bytes} bytes · {ms} ms",
  "playground.convertFailed": "Convert failed: {reason}",
  "shortcuts.title": "Keyboard shortcuts",
  "shortcuts.openTitle": "Keyboard shortcuts",
  "shortcuts.groupNavigation": "Navigation",
  "shortcuts.groupView": "View",
  "shortcuts.groupSelection": "Selection",
  "shortcuts.groupOutput": "Search & output",
  "shortcuts.prevSlide": "Previous slide",
  "shortcuts.nextSlide": "Next slide",
  "shortcuts.firstSlide": "First slide",
  "shortcuts.lastSlide": "Last slide",
  "shortcuts.zoomIn": "Zoom in",
  "shortcuts.zoomOut": "Zoom out",
  "shortcuts.zoomReset": "Reset zoom to 100%",
  "shortcuts.panSlide": "Pan slide while held",
  "shortcuts.click": "Click",
  "shortcuts.drag": "Drag",
  "shortcuts.doubleClick": "Double-click",
  "shortcuts.selectShape": "Select a shape",
  "shortcuts.toggleSelect": "Toggle shape in selection",
  "shortcuts.rubberBand": "Rubber-band select",
  "shortcuts.selectAll": "Select every shape",
  "shortcuts.copyText": "Copy selected text",
  "shortcuts.editText": "Enter text-edit mode",
  "shortcuts.clearSelection": "Clear selection / exit",
  "shortcuts.toggleSearch": "Find in slides",
  "shortcuts.print": "Print deck",
};

const KO: Catalog = {
  _locale: "ko",
  "common.close": "닫기",
  "common.cancel": "취소",
  "common.loading": "로드 중…",
  "common.ready": "준비됨.",
  "common.bytes": "{count} 바이트",
  "nav.firstSlide": "첫 슬라이드",
  "nav.previousSlide": "이전 (←)",
  "nav.nextSlide": "다음 (→)",
  "nav.lastSlide": "마지막 슬라이드",
  "nav.slideCounter": "{current} / {total}",
  "nav.slideCounterEmpty": "—",
  "search.button": "검색",
  "search.placeholder": "슬라이드 내 검색…",
  "search.title": "검색 (Cmd/Ctrl+F)",
  "search.empty": "검색",
  "search.noMatches": "결과 없음.",
  "search.typeToSearch": "검색어를 입력하세요.",
  "output.print": "인쇄",
  "output.printTitle": "인쇄 (Cmd/Ctrl+P)",
  "output.pdf": "PDF",
  "output.pdfTitle": "PDF로 내보내기",
  "output.slideshow": "슬라이드쇼",
  "output.slideshowTitle": "슬라이드쇼 시작 (F5)",
  "output.gateLoadFirst": "먼저 파일을 여세요",
  "output.gatePreparing": "슬라이드 준비 중 — {current} / {total} 완료",
  "settings.title": "설정",
  "settings.openTitle": "설정",
  "file.open": "열기",
  "render.label": "텍스트 렌더링:",
  "render.text": "벡터 텍스트",
  "render.path": "경로 외곽선",
  "render.auto": "자동",
  "status.toggleNotes": "노트 창 토글",
  "status.resizeSidebar": "썸네일 사이드바 너비 조절 (드래그)",
  "status.normalView": "기본 보기 (슬라이드 + 썸네일)",
  "status.gridView": "여러 슬라이드 보기",
  "status.zoomOut": "축소",
  "status.zoomIn": "확대",
  "status.zoomReset": "100%로 재설정",
  "status.fitWindow": "창에 맞춤",
  "status.zoom": "확대/축소",
  "status.slideOf": "슬라이드 {current} / {total}",
  "status.slideEmpty": "—",
  "status.selectionFontLabel": "폰트:",
  "status.selectionFontMultiple": "{first} 외 {extra}개",
  "status.selectionFontTitle": "선택 항목에 사용된 폰트: {fonts}",
  "fontUsage.title": "폰트 매핑",
  "fontUsage.close": "닫기",
  "fontUsage.headerRequested": "원본 폰트",
  "fontUsage.headerEffective": "렌더 폰트",
  "fontUsage.systemFallback": "시스템 기본 폰트",
  "fontUsage.allMatched": "모든 원본 폰트 사용 중",
  "fontUsage.substituteCount": "{count}개 폰트 대체 중",
  "notes.heading": "노트 — 슬라이드 {current}",
  "notes.headingWithSection": "노트 — 슬라이드 {current} · {section}",
  "notes.empty": "이 슬라이드의 노트가 없습니다.",
  "notes.standaloneHeading": "노트 — 슬라이드 {current}",
  "notes.standaloneEmpty": "이 슬라이드의 발표자 노트가 없습니다.",
  "notes.layoutLabel": "레이아웃: {value}",
  "notes.sectionLabel": "섹션: {value}",
  "notes.noSlide": "로드된 슬라이드 없음.",
  "section.empty": "섹션 없음",
  "dialog.title": "설정",
  "dialog.appearance": "모양",
  "dialog.theme": "테마",
  "dialog.themeDesc":
    "슬라이드, 리본, 사이드바, 대화 상자를 포함한 뷰어 전체에 적용됩니다.",
  "dialog.themeAuto": "자동",
  "dialog.themeAutoDesc": "OS / 브라우저 따라가기",
  "dialog.themeLight": "라이트",
  "dialog.themeLightDesc": "항상 밝게",
  "dialog.themeDark": "다크",
  "dialog.themeDarkDesc": "항상 어둡게",
  "dialog.themeHighContrast": "고대비",
  "dialog.themeHighContrastDesc": "접근성",
  "dialog.ruler": "눈금자",
  "dialog.rulerShow": "슬라이드 주변에 눈금자 표시",
  "dialog.rulerUnitLabel": "단위",
  "dialog.rulerUnitDesc":
    "PowerPoint 기본은 슬라이드를 0에 중심을 두고 cm 눈금을 표시합니다. 픽셀 모드는 슬라이드 가장자리에서 시작하는 화면 좌표를 보여줍니다.",
  "dialog.rulerUnitCm": "센티미터 (cm)",
  "dialog.rulerUnitCmDesc": "PowerPoint 기본값",
  "dialog.rulerUnitPx": "픽셀 (px)",
  "dialog.rulerUnitPxDesc": "화면 픽셀",
  "dialog.language": "언어",
  "dialog.languageDesc":
    "뷰어 인터페이스 언어를 선택합니다. 자동은 브라우저/OS 기본값을 따릅니다.",
  "dialog.languageAuto": "자동 ({detected})",
  "dialog.about": "정보",
  "dialog.aboutAppName": "앱",
  "dialog.aboutVersion": "버전",
  "dialog.aboutRendering": "환경",
  "dialog.aboutRenderingValue": "WebAssembly · 브라우저 전용 · 오프라인 지원",
  "dialog.aboutEngine": "엔진",
  "dialog.aboutNpmPackage": "프론트엔드 (npm)",
  "dialog.aboutPackage": "패키지",
  "dialog.aboutLicense": "라이선스",
  "dialog.aboutCopyright": "저작권",
  "dialog.aboutDeveloper": "개발자",
  "dialog.aboutRepository": "저장소",
  "dialog.viewerSettingsAriaLabel": "뷰어 설정",
  "phase.preparingSlide": "슬라이드 준비 중 {current} / {total}…",
  "phase.renderingPdf": "PDF 페이지 렌더링 {current} / {total}…",
  "phase.encodingPdf": "PDF 인코딩 중…",
  "phase.preparingSlides": "슬라이드 준비 중…",
  "status.loadedSlides": "{count}개 슬라이드 로드됨",
  "status.nothingToPrint": "인쇄할 내용 없음.",
  "status.nothingToExport": "내보낼 내용 없음.",
  "status.pdfFailed": "PDF 내보내기 실패: {reason}",
  "status.exported": "{count}개 슬라이드를 PDF로 내보냈습니다.",
  "status.errorPrefix": "[오류:{phase}] {message}",
  "viewer.ariaLabel": "PPTX 뷰어",
  "viewer.noFile": "(파일 없음)",
  "viewer.slideTitle": "슬라이드 {number}",
  "viewer.empty": ".pptx 파일을 여세요.",
  "viewer.openFile": "파일 열기",
  "phase.preparingSlideOf": "슬라이드 준비 중 {current} / {total}",
  "phase.preparingPdf": "PDF 준비 중…",
  "phase.preparingPrint": "인쇄 준비 중…",
  "phase.layingOutPrintOf": "페이지 배치 중 {current} / {total}…",
  "phase.openingPrintDialog": "인쇄 창 여는 중…",
  "phase.savingPdf": "PDF 저장 중…",
  "progress.titlePrint": "인쇄 준비",
  "progress.titlePdf": "PDF 내보내기",
  "viewer.loading": "로드 중…",
  "viewer.error": "⚠ {message}",
  "playground.title": "slideglance 플레이그라운드",
  "playground.upload": "업로드:",
  "playground.samples": "샘플:",
  "playground.pickPrompt": "샘플을 선택하거나 .pptx를 업로드하세요.",
  "playground.loadingFile": "{name} 로드 중…",
  "playground.failedHttp": "실패: HTTP {status}",
  "playground.fileInfo": "{name} · {bytes} 바이트 · {ms} ms",
  "playground.convertFailed": "변환 실패: {reason}",
  "shortcuts.title": "키보드 단축키",
  "shortcuts.openTitle": "키보드 단축키",
  "shortcuts.groupNavigation": "탐색",
  "shortcuts.groupView": "보기",
  "shortcuts.groupSelection": "선택",
  "shortcuts.groupOutput": "검색·출력",
  "shortcuts.prevSlide": "이전 슬라이드",
  "shortcuts.nextSlide": "다음 슬라이드",
  "shortcuts.firstSlide": "첫 슬라이드",
  "shortcuts.lastSlide": "마지막 슬라이드",
  "shortcuts.zoomIn": "확대",
  "shortcuts.zoomOut": "축소",
  "shortcuts.zoomReset": "100%로 재설정",
  "shortcuts.panSlide": "슬라이드 이동 (누르고 드래그)",
  "shortcuts.click": "클릭",
  "shortcuts.drag": "드래그",
  "shortcuts.doubleClick": "더블 클릭",
  "shortcuts.selectShape": "도형 선택",
  "shortcuts.toggleSelect": "선택 토글",
  "shortcuts.rubberBand": "드래그 선택",
  "shortcuts.selectAll": "모든 도형 선택",
  "shortcuts.copyText": "선택한 텍스트 복사",
  "shortcuts.editText": "텍스트 편집 모드",
  "shortcuts.clearSelection": "선택 해제 / 종료",
  "shortcuts.toggleSearch": "슬라이드 검색",
  "shortcuts.print": "인쇄",
};

const JA: Catalog = {
  _locale: "ja",
  "common.close": "閉じる",
  "common.cancel": "キャンセル",
  "common.loading": "読み込み中…",
  "common.ready": "準備完了。",
  "common.bytes": "{count} バイト",
  "nav.firstSlide": "最初のスライド",
  "nav.previousSlide": "前へ (←)",
  "nav.nextSlide": "次へ (→)",
  "nav.lastSlide": "最後のスライド",
  "nav.slideCounter": "{current} / {total}",
  "nav.slideCounterEmpty": "—",
  "search.button": "検索",
  "search.placeholder": "スライド内を検索…",
  "search.title": "検索 (Cmd/Ctrl+F)",
  "search.empty": "検索",
  "search.noMatches": "一致なし。",
  "search.typeToSearch": "検索語を入力。",
  "output.print": "印刷",
  "output.printTitle": "印刷 (Cmd/Ctrl+P)",
  "output.pdf": "PDF",
  "output.pdfTitle": "PDF にエクスポート",
  "output.slideshow": "スライドショー",
  "output.slideshowTitle": "スライドショー開始 (F5)",
  "output.gateLoadFirst": "先にデッキを読み込みます",
  "output.gatePreparing": "スライド準備中 — {current} / {total} 完了",
  "settings.title": "設定",
  "settings.openTitle": "設定",
  "file.open": "開く",
  "render.label": "テキスト描画:",
  "render.text": "ベクターテキスト",
  "render.path": "パスアウトライン",
  "render.auto": "自動",
  "status.toggleNotes": "ノートペインの切替",
  "status.resizeSidebar": "サムネイルサイドバーの幅を変更 (ドラッグ)",
  "status.normalView": "標準表示 (スライド + サムネイル)",
  "status.gridView": "スライド一覧表示",
  "status.zoomOut": "縮小",
  "status.zoomIn": "拡大",
  "status.zoomReset": "100% にリセット",
  "status.fitWindow": "ウィンドウに合わせる",
  "status.zoom": "ズーム",
  "status.slideOf": "スライド {current} / {total}",
  "status.slideEmpty": "—",
  "status.selectionFontLabel": "フォント:",
  "status.selectionFontMultiple": "{first} 他 {extra}",
  "status.selectionFontTitle": "選択範囲で使用されているフォント: {fonts}",
  "fontUsage.title": "フォントマッピング",
  "fontUsage.close": "閉じる",
  "fontUsage.headerRequested": "指定フォント",
  "fontUsage.headerEffective": "実際の描画",
  "fontUsage.systemFallback": "システム既定",
  "fontUsage.allMatched": "すべての指定フォントが利用可能",
  "fontUsage.substituteCount": "{count}件のフォントを代替",
  "notes.heading": "ノート — スライド {current}",
  "notes.headingWithSection": "ノート — スライド {current} · {section}",
  "notes.empty": "このスライドのノートはありません。",
  "notes.standaloneHeading": "ノート — スライド {current}",
  "notes.standaloneEmpty": "このスライドの発表者ノートはありません。",
  "notes.layoutLabel": "レイアウト: {value}",
  "notes.sectionLabel": "セクション: {value}",
  "notes.noSlide": "スライドが読み込まれていません。",
  "section.empty": "セクションなし",
  "dialog.title": "設定",
  "dialog.appearance": "外観",
  "dialog.theme": "テーマ",
  "dialog.themeDesc":
    "スライド、リボン、サイドバー、ダイアログを含むビューワ全体に適用されます。",
  "dialog.themeAuto": "自動",
  "dialog.themeAutoDesc": "OS / ブラウザに従う",
  "dialog.themeLight": "ライト",
  "dialog.themeLightDesc": "常にライト",
  "dialog.themeDark": "ダーク",
  "dialog.themeDarkDesc": "常にダーク",
  "dialog.themeHighContrast": "ハイコントラスト",
  "dialog.themeHighContrastDesc": "アクセシビリティ",
  "dialog.ruler": "ルーラー",
  "dialog.rulerShow": "スライド周囲にルーラーを表示",
  "dialog.rulerUnitLabel": "単位",
  "dialog.rulerUnitDesc":
    "PowerPoint の既定ではスライドを 0 中心に配置し cm 目盛を表示します。ピクセルモードはスライド端から始まる画面ピクセル座標です。",
  "dialog.rulerUnitCm": "センチメートル (cm)",
  "dialog.rulerUnitCmDesc": "PowerPoint 既定",
  "dialog.rulerUnitPx": "ピクセル (px)",
  "dialog.rulerUnitPxDesc": "画面ピクセル",
  "dialog.language": "言語",
  "dialog.languageDesc":
    "ビューワの言語を選択します。自動はブラウザ / OS の設定に従います。",
  "dialog.languageAuto": "自動 ({detected})",
  "dialog.about": "情報",
  "dialog.aboutAppName": "アプリ",
  "dialog.aboutEngine": "エンジン",
  "dialog.aboutNpmPackage": "フロントエンド (npm)",
  "dialog.aboutCopyright": "著作権",
  "dialog.aboutDeveloper": "開発者",
  "dialog.aboutRepository": "リポジトリ",
  "dialog.aboutPackage": "パッケージ",
  "dialog.aboutVersion": "バージョン",
  "dialog.aboutRendering": "環境",
  "dialog.aboutRenderingValue": "WebAssembly · ブラウザのみ · オフライン対応",
  "dialog.aboutLicense": "ライセンス",
  "dialog.viewerSettingsAriaLabel": "ビューワ設定",
  "phase.preparingSlide": "スライド準備中 {current} / {total}…",
  "phase.renderingPdf": "PDF ページ描画 {current} / {total}…",
  "phase.encodingPdf": "PDF エンコード中…",
  "phase.preparingSlides": "スライド準備中…",
  "status.loadedSlides": "{count} 枚のスライドを読み込みました",
  "status.nothingToPrint": "印刷対象なし。",
  "status.nothingToExport": "エクスポート対象なし。",
  "status.pdfFailed": "PDF エクスポート失敗: {reason}",
  "status.exported": "{count} 枚のスライドを PDF にエクスポートしました。",
  "status.errorPrefix": "[エラー:{phase}] {message}",
  "viewer.ariaLabel": "PPTX ビューワ",
  "viewer.noFile": "(ファイルなし)",
  "viewer.slideTitle": "スライド {number}",
  "viewer.empty": ".pptx ファイルを開いてください。",
  "viewer.openFile": "ファイルを開く",
  "phase.preparingSlideOf": "スライド準備中 {current} / {total}",
  "phase.preparingPdf": "PDF を準備中…",
  "phase.preparingPrint": "印刷を準備中…",
  "phase.layingOutPrintOf": "ページ配置中 {current} / {total}…",
  "phase.openingPrintDialog": "印刷ダイアログを開いています…",
  "phase.savingPdf": "PDF を保存中…",
  "progress.titlePrint": "印刷の準備",
  "progress.titlePdf": "PDF 書き出し",
  "viewer.loading": "読み込み中…",
  "viewer.error": "⚠ {message}",
  "playground.title": "slideglance プレイグラウンド",
  "playground.upload": "アップロード:",
  "playground.samples": "サンプル:",
  "playground.pickPrompt": "サンプルを選ぶか .pptx をアップロード。",
  "playground.loadingFile": "{name} 読み込み中…",
  "playground.failedHttp": "失敗: HTTP {status}",
  "playground.fileInfo": "{name} · {bytes} バイト · {ms} ms",
  "playground.convertFailed": "変換失敗: {reason}",
  "shortcuts.title": "キーボードショートカット",
  "shortcuts.openTitle": "キーボードショートカット",
  "shortcuts.groupNavigation": "ナビゲーション",
  "shortcuts.groupView": "表示",
  "shortcuts.groupSelection": "選択",
  "shortcuts.groupOutput": "検索・出力",
  "shortcuts.prevSlide": "前のスライド",
  "shortcuts.nextSlide": "次のスライド",
  "shortcuts.firstSlide": "最初のスライド",
  "shortcuts.lastSlide": "最後のスライド",
  "shortcuts.zoomIn": "拡大",
  "shortcuts.zoomOut": "縮小",
  "shortcuts.zoomReset": "100%にリセット",
  "shortcuts.panSlide": "スライドをドラッグで移動",
  "shortcuts.click": "クリック",
  "shortcuts.drag": "ドラッグ",
  "shortcuts.doubleClick": "ダブルクリック",
  "shortcuts.selectShape": "図形を選択",
  "shortcuts.toggleSelect": "選択の切り替え",
  "shortcuts.rubberBand": "ドラッグ選択",
  "shortcuts.selectAll": "すべての図形を選択",
  "shortcuts.copyText": "選択テキストをコピー",
  "shortcuts.editText": "テキスト編集モード",
  "shortcuts.clearSelection": "選択解除 / 終了",
  "shortcuts.toggleSearch": "スライド内検索",
  "shortcuts.print": "印刷",
};

const ZH_CN: Catalog = {
  _locale: "zh-CN",
  "common.close": "关闭",
  "common.cancel": "取消",
  "common.loading": "加载中…",
  "common.ready": "就绪。",
  "common.bytes": "{count} 字节",
  "nav.firstSlide": "第一张幻灯片",
  "nav.previousSlide": "上一张 (←)",
  "nav.nextSlide": "下一张 (→)",
  "nav.lastSlide": "最后一张幻灯片",
  "nav.slideCounter": "{current} / {total}",
  "nav.slideCounterEmpty": "—",
  "search.button": "搜索",
  "search.placeholder": "在幻灯片中查找…",
  "search.title": "搜索 (Cmd/Ctrl+F)",
  "search.empty": "搜索",
  "search.noMatches": "无匹配。",
  "search.typeToSearch": "输入以搜索。",
  "output.print": "打印",
  "output.printTitle": "打印 (Cmd/Ctrl+P)",
  "output.pdf": "PDF",
  "output.pdfTitle": "导出为 PDF",
  "output.slideshow": "幻灯片放映",
  "output.slideshowTitle": "开始放映 (F5)",
  "output.gateLoadFirst": "请先加载文件",
  "output.gatePreparing": "正在准备幻灯片 — {current} / {total} 已完成",
  "settings.title": "设置",
  "settings.openTitle": "设置",
  "file.open": "打开",
  "render.label": "文本渲染:",
  "render.text": "矢量文本",
  "render.path": "路径轮廓",
  "render.auto": "自动",
  "status.toggleNotes": "切换备注窗格",
  "status.resizeSidebar": "拖动调整缩略图侧栏宽度",
  "status.normalView": "普通视图 (幻灯片 + 缩略图)",
  "status.gridView": "幻灯片浏览视图",
  "status.zoomOut": "缩小",
  "status.zoomIn": "放大",
  "status.zoomReset": "重置为 100%",
  "status.fitWindow": "适合窗口",
  "status.zoom": "缩放",
  "status.slideOf": "幻灯片 {current} / {total}",
  "status.slideEmpty": "—",
  "status.selectionFontLabel": "字体:",
  "status.selectionFontMultiple": "{first} 等 {extra} 个",
  "status.selectionFontTitle": "所选项目使用的字体: {fonts}",
  "fontUsage.title": "字体映射",
  "fontUsage.close": "关闭",
  "fontUsage.headerRequested": "原始字体",
  "fontUsage.headerEffective": "实际渲染",
  "fontUsage.systemFallback": "系统默认字体",
  "fontUsage.allMatched": "所有原始字体均可用",
  "fontUsage.substituteCount": "{count} 个字体已替换",
  "notes.heading": "备注 — 幻灯片 {current}",
  "notes.headingWithSection": "备注 — 幻灯片 {current} · {section}",
  "notes.empty": "此幻灯片无备注。",
  "notes.standaloneHeading": "备注 — 幻灯片 {current}",
  "notes.standaloneEmpty": "此幻灯片无演讲者备注。",
  "notes.layoutLabel": "版式: {value}",
  "notes.sectionLabel": "节: {value}",
  "notes.noSlide": "未加载幻灯片。",
  "section.empty": "无节",
  "dialog.title": "设置",
  "dialog.appearance": "外观",
  "dialog.theme": "主题",
  "dialog.themeDesc": "应用于整个查看器,包括幻灯片、功能区、侧栏和对话框。",
  "dialog.themeAuto": "自动",
  "dialog.themeAutoDesc": "跟随 OS / 浏览器",
  "dialog.themeLight": "浅色",
  "dialog.themeLightDesc": "始终浅色",
  "dialog.themeDark": "深色",
  "dialog.themeDarkDesc": "始终深色",
  "dialog.themeHighContrast": "高对比度",
  "dialog.themeHighContrastDesc": "无障碍",
  "dialog.ruler": "标尺",
  "dialog.rulerShow": "在幻灯片周围显示标尺",
  "dialog.rulerUnitLabel": "单位",
  "dialog.rulerUnitDesc":
    "PowerPoint 默认以 0 居中并使用厘米刻度。像素模式从幻灯片边缘开始显示屏幕坐标。",
  "dialog.rulerUnitCm": "厘米 (cm)",
  "dialog.rulerUnitCmDesc": "PowerPoint 默认",
  "dialog.rulerUnitPx": "像素 (px)",
  "dialog.rulerUnitPxDesc": "屏幕像素",
  "dialog.language": "语言",
  "dialog.languageDesc": "选择查看器界面语言。自动会跟随您的浏览器/系统设置。",
  "dialog.languageAuto": "自动 ({detected})",
  "dialog.about": "关于",
  "dialog.aboutAppName": "应用",
  "dialog.aboutEngine": "引擎",
  "dialog.aboutNpmPackage": "前端 (npm)",
  "dialog.aboutCopyright": "版权",
  "dialog.aboutDeveloper": "开发者",
  "dialog.aboutRepository": "代码仓库",
  "dialog.aboutPackage": "包",
  "dialog.aboutVersion": "版本",
  "dialog.aboutRendering": "环境",
  "dialog.aboutRenderingValue": "WebAssembly · 仅浏览器 · 支持离线",
  "dialog.aboutLicense": "许可",
  "dialog.viewerSettingsAriaLabel": "查看器设置",
  "phase.preparingSlide": "正在准备幻灯片 {current} / {total}…",
  "phase.renderingPdf": "正在渲染 PDF 页面 {current} / {total}…",
  "phase.encodingPdf": "正在编码 PDF…",
  "phase.preparingSlides": "正在准备幻灯片…",
  "status.loadedSlides": "已加载 {count} 张幻灯片",
  "status.nothingToPrint": "无可打印内容。",
  "status.nothingToExport": "无可导出内容。",
  "status.pdfFailed": "PDF 导出失败: {reason}",
  "status.exported": "已导出 {count} 张幻灯片为 PDF。",
  "status.errorPrefix": "[错误:{phase}] {message}",
  "viewer.ariaLabel": "PPTX 查看器",
  "viewer.noFile": "(无文件)",
  "viewer.slideTitle": "幻灯片 {number}",
  "viewer.empty": "请打开 .pptx 文件。",
  "viewer.openFile": "打开文件",
  "phase.preparingSlideOf": "正在准备幻灯片 {current} / {total}",
  "phase.preparingPdf": "正在准备 PDF…",
  "phase.preparingPrint": "正在准备打印…",
  "phase.layingOutPrintOf": "正在排版页面 {current} / {total}…",
  "phase.openingPrintDialog": "正在打开打印对话框…",
  "phase.savingPdf": "正在保存 PDF…",
  "progress.titlePrint": "准备打印",
  "progress.titlePdf": "导出 PDF",
  "viewer.loading": "加载中…",
  "viewer.error": "⚠ {message}",
  "playground.title": "slideglance 演练场",
  "playground.upload": "上传:",
  "playground.samples": "示例:",
  "playground.pickPrompt": "选择示例或上传 .pptx。",
  "playground.loadingFile": "正在加载 {name}…",
  "playground.failedHttp": "失败: HTTP {status}",
  "playground.fileInfo": "{name} · {bytes} 字节 · {ms} 毫秒",
  "playground.convertFailed": "转换失败: {reason}",
  "shortcuts.title": "键盘快捷键",
  "shortcuts.openTitle": "键盘快捷键",
  "shortcuts.groupNavigation": "导航",
  "shortcuts.groupView": "视图",
  "shortcuts.groupSelection": "选择",
  "shortcuts.groupOutput": "搜索·输出",
  "shortcuts.prevSlide": "上一张",
  "shortcuts.nextSlide": "下一张",
  "shortcuts.firstSlide": "第一张",
  "shortcuts.lastSlide": "最后一张",
  "shortcuts.zoomIn": "放大",
  "shortcuts.zoomOut": "缩小",
  "shortcuts.zoomReset": "重置到100%",
  "shortcuts.panSlide": "按住并拖动平移",
  "shortcuts.click": "点击",
  "shortcuts.drag": "拖动",
  "shortcuts.doubleClick": "双击",
  "shortcuts.selectShape": "选择形状",
  "shortcuts.toggleSelect": "切换选中",
  "shortcuts.rubberBand": "框选",
  "shortcuts.selectAll": "全选",
  "shortcuts.copyText": "复制选中文字",
  "shortcuts.editText": "进入文字编辑",
  "shortcuts.clearSelection": "清除选择 / 退出",
  "shortcuts.toggleSearch": "搜索幻灯片",
  "shortcuts.print": "打印",
};

const ZH_TW: Catalog = {
  _locale: "zh-TW",
  "common.close": "關閉",
  "common.cancel": "取消",
  "common.loading": "載入中…",
  "common.ready": "就緒。",
  "common.bytes": "{count} 位元組",
  "nav.firstSlide": "第一張投影片",
  "nav.previousSlide": "上一張 (←)",
  "nav.nextSlide": "下一張 (→)",
  "nav.lastSlide": "最後一張投影片",
  "nav.slideCounter": "{current} / {total}",
  "nav.slideCounterEmpty": "—",
  "search.button": "搜尋",
  "search.placeholder": "在投影片中尋找…",
  "search.title": "搜尋 (Cmd/Ctrl+F)",
  "search.empty": "搜尋",
  "search.noMatches": "無符合項目。",
  "search.typeToSearch": "輸入以搜尋。",
  "output.print": "列印",
  "output.printTitle": "列印 (Cmd/Ctrl+P)",
  "output.pdf": "PDF",
  "output.pdfTitle": "匯出為 PDF",
  "output.slideshow": "投影片放映",
  "output.slideshowTitle": "開始放映 (F5)",
  "output.gateLoadFirst": "請先載入檔案",
  "output.gatePreparing": "投影片準備中 — {current} / {total} 完成",
  "settings.title": "設定",
  "settings.openTitle": "設定",
  "file.open": "開啟",
  "render.label": "文字渲染:",
  "render.text": "向量文字",
  "render.path": "路徑外框",
  "render.auto": "自動",
  "status.toggleNotes": "切換備忘稿窗格",
  "status.resizeSidebar": "拖曳調整縮圖側欄寬度",
  "status.normalView": "標準檢視 (投影片 + 縮圖)",
  "status.gridView": "投影片瀏覽檢視",
  "status.zoomOut": "縮小",
  "status.zoomIn": "放大",
  "status.zoomReset": "重設為 100%",
  "status.fitWindow": "符合視窗",
  "status.zoom": "縮放",
  "status.slideOf": "投影片 {current} / {total}",
  "status.slideEmpty": "—",
  "status.selectionFontLabel": "字型:",
  "status.selectionFontMultiple": "{first} 等 {extra} 個",
  "status.selectionFontTitle": "所選項目使用的字型: {fonts}",
  "fontUsage.title": "字型對應",
  "fontUsage.close": "關閉",
  "fontUsage.headerRequested": "原始字型",
  "fontUsage.headerEffective": "實際渲染",
  "fontUsage.systemFallback": "系統預設字型",
  "fontUsage.allMatched": "所有原始字型皆可用",
  "fontUsage.substituteCount": "{count} 個字型已替換",
  "notes.heading": "備忘稿 — 投影片 {current}",
  "notes.headingWithSection": "備忘稿 — 投影片 {current} · {section}",
  "notes.empty": "此投影片無備忘稿。",
  "notes.standaloneHeading": "備忘稿 — 投影片 {current}",
  "notes.standaloneEmpty": "此投影片無演講者備忘稿。",
  "notes.layoutLabel": "版面配置: {value}",
  "notes.sectionLabel": "區段: {value}",
  "notes.noSlide": "未載入投影片。",
  "section.empty": "無區段",
  "dialog.title": "設定",
  "dialog.appearance": "外觀",
  "dialog.theme": "佈景主題",
  "dialog.themeDesc": "套用至整個檢視器,包含投影片、功能區、側邊欄與對話框。",
  "dialog.themeAuto": "自動",
  "dialog.themeAutoDesc": "依 OS / 瀏覽器",
  "dialog.themeLight": "淺色",
  "dialog.themeLightDesc": "永遠淺色",
  "dialog.themeDark": "深色",
  "dialog.themeDarkDesc": "永遠深色",
  "dialog.themeHighContrast": "高對比",
  "dialog.themeHighContrastDesc": "輔助使用",
  "dialog.ruler": "尺規",
  "dialog.rulerShow": "在投影片周圍顯示尺規",
  "dialog.rulerUnitLabel": "單位",
  "dialog.rulerUnitDesc":
    "PowerPoint 預設以 0 為中心並顯示公分刻度。像素模式從投影片邊緣開始顯示螢幕座標。",
  "dialog.rulerUnitCm": "公分 (cm)",
  "dialog.rulerUnitCmDesc": "PowerPoint 預設",
  "dialog.rulerUnitPx": "像素 (px)",
  "dialog.rulerUnitPxDesc": "螢幕像素",
  "dialog.language": "語言",
  "dialog.languageDesc":
    "選擇檢視器介面語言。自動會跟隨瀏覽器 / 作業系統設定。",
  "dialog.languageAuto": "自動 ({detected})",
  "dialog.about": "關於",
  "dialog.aboutAppName": "應用",
  "dialog.aboutEngine": "引擎",
  "dialog.aboutNpmPackage": "前端 (npm)",
  "dialog.aboutCopyright": "版權",
  "dialog.aboutDeveloper": "開發者",
  "dialog.aboutRepository": "程式庫",
  "dialog.aboutPackage": "套件",
  "dialog.aboutVersion": "版本",
  "dialog.aboutRendering": "環境",
  "dialog.aboutRenderingValue": "WebAssembly · 僅瀏覽器 · 支援離線",
  "dialog.aboutLicense": "授權",
  "dialog.viewerSettingsAriaLabel": "檢視器設定",
  "phase.preparingSlide": "投影片準備中 {current} / {total}…",
  "phase.renderingPdf": "PDF 頁面渲染中 {current} / {total}…",
  "phase.encodingPdf": "PDF 編碼中…",
  "phase.preparingSlides": "投影片準備中…",
  "status.loadedSlides": "已載入 {count} 張投影片",
  "status.nothingToPrint": "無可列印內容。",
  "status.nothingToExport": "無可匯出內容。",
  "status.pdfFailed": "PDF 匯出失敗: {reason}",
  "status.exported": "已將 {count} 張投影片匯出為 PDF。",
  "status.errorPrefix": "[錯誤:{phase}] {message}",
  "viewer.ariaLabel": "PPTX 檢視器",
  "viewer.noFile": "(無檔案)",
  "viewer.slideTitle": "投影片 {number}",
  "viewer.empty": "請開啟 .pptx 檔案。",
  "viewer.openFile": "開啟檔案",
  "phase.preparingSlideOf": "正在準備投影片 {current} / {total}",
  "phase.preparingPdf": "正在準備 PDF…",
  "phase.preparingPrint": "正在準備列印…",
  "phase.layingOutPrintOf": "正在排版頁面 {current} / {total}…",
  "phase.openingPrintDialog": "正在開啟列印對話框…",
  "phase.savingPdf": "正在儲存 PDF…",
  "progress.titlePrint": "準備列印",
  "progress.titlePdf": "匯出 PDF",
  "viewer.loading": "載入中…",
  "viewer.error": "⚠ {message}",
  "playground.title": "slideglance 試驗場",
  "playground.upload": "上傳:",
  "playground.samples": "範例:",
  "playground.pickPrompt": "選擇範例或上傳 .pptx。",
  "playground.loadingFile": "正在載入 {name}…",
  "playground.failedHttp": "失敗: HTTP {status}",
  "playground.fileInfo": "{name} · {bytes} 位元組 · {ms} 毫秒",
  "playground.convertFailed": "轉換失敗: {reason}",
  "shortcuts.title": "鍵盤捷徑",
  "shortcuts.openTitle": "鍵盤捷徑",
  "shortcuts.groupNavigation": "導覽",
  "shortcuts.groupView": "檢視",
  "shortcuts.groupSelection": "選取",
  "shortcuts.groupOutput": "搜尋·輸出",
  "shortcuts.prevSlide": "上一張",
  "shortcuts.nextSlide": "下一張",
  "shortcuts.firstSlide": "第一張",
  "shortcuts.lastSlide": "最後一張",
  "shortcuts.zoomIn": "放大",
  "shortcuts.zoomOut": "縮小",
  "shortcuts.zoomReset": "重設為100%",
  "shortcuts.panSlide": "按住並拖曳平移",
  "shortcuts.click": "按一下",
  "shortcuts.drag": "拖曳",
  "shortcuts.doubleClick": "按兩下",
  "shortcuts.selectShape": "選取圖形",
  "shortcuts.toggleSelect": "切換選取",
  "shortcuts.rubberBand": "框選",
  "shortcuts.selectAll": "全選",
  "shortcuts.copyText": "複製選取文字",
  "shortcuts.editText": "進入文字編輯",
  "shortcuts.clearSelection": "清除選取 / 結束",
  "shortcuts.toggleSearch": "搜尋投影片",
  "shortcuts.print": "列印",
};

const ES: Catalog = {
  _locale: "es",
  "common.close": "Cerrar",
  "common.cancel": "Cancelar",
  "common.loading": "Cargando…",
  "common.ready": "Listo.",
  "common.bytes": "{count} bytes",
  "nav.firstSlide": "Primera diapositiva",
  "nav.previousSlide": "Anterior (←)",
  "nav.nextSlide": "Siguiente (→)",
  "nav.lastSlide": "Última diapositiva",
  "nav.slideCounter": "{current} / {total}",
  "nav.slideCounterEmpty": "—",
  "search.button": "Buscar",
  "search.placeholder": "Buscar en diapositivas…",
  "search.title": "Buscar (Cmd/Ctrl+F)",
  "search.empty": "Buscar",
  "search.noMatches": "Sin coincidencias.",
  "search.typeToSearch": "Escribe para buscar.",
  "output.print": "Imprimir",
  "output.printTitle": "Imprimir (Cmd/Ctrl+P)",
  "output.pdf": "PDF",
  "output.pdfTitle": "Exportar a PDF",
  "output.slideshow": "Presentación",
  "output.slideshowTitle": "Iniciar presentación (F5)",
  "output.gateLoadFirst": "Carga un archivo primero",
  "output.gatePreparing":
    "Preparando diapositivas — {current} / {total} listas",
  "settings.title": "Ajustes",
  "settings.openTitle": "Ajustes",
  "file.open": "Abrir",
  "render.label": "Renderizado de texto:",
  "render.text": "Texto vectorial",
  "render.path": "Contorno de ruta",
  "render.auto": "Automático",
  "status.toggleNotes": "Alternar panel de notas",
  "status.resizeSidebar": "Arrastrar para redimensionar el panel de miniaturas",
  "status.normalView": "Vista normal (diapositiva + miniaturas)",
  "status.gridView": "Clasificador de diapositivas",
  "status.zoomOut": "Alejar",
  "status.zoomIn": "Acercar",
  "status.zoomReset": "Restablecer a 100%",
  "status.fitWindow": "Ajustar a la ventana",
  "status.zoom": "Zoom",
  "status.slideOf": "diapositiva {current} / {total}",
  "status.slideEmpty": "—",
  "status.selectionFontLabel": "Fuente:",
  "status.selectionFontMultiple": "{first} y {extra} más",
  "status.selectionFontTitle": "Fuente(s) usada(s) por la selección: {fonts}",
  "fontUsage.title": "Asignación de fuentes",
  "fontUsage.close": "Cerrar",
  "fontUsage.headerRequested": "Fuente original",
  "fontUsage.headerEffective": "Renderizada como",
  "fontUsage.systemFallback": "fuente del sistema",
  "fontUsage.allMatched": "Todas las fuentes originales disponibles",
  "fontUsage.substituteCount": "{count} fuente(s) sustituida(s)",
  "notes.heading": "Notas — diapositiva {current}",
  "notes.headingWithSection": "Notas — diapositiva {current} · {section}",
  "notes.empty": "Sin notas para esta diapositiva.",
  "notes.standaloneHeading": "Notas — diapositiva {current}",
  "notes.standaloneEmpty": "Sin notas del orador para esta diapositiva.",
  "notes.layoutLabel": "diseño: {value}",
  "notes.sectionLabel": "sección: {value}",
  "notes.noSlide": "Ninguna diapositiva cargada.",
  "section.empty": "sin secciones",
  "dialog.title": "Ajustes",
  "dialog.appearance": "Apariencia",
  "dialog.theme": "Tema",
  "dialog.themeDesc":
    "Se aplica a todo el visor, incluyendo el escenario, la cinta, la barra lateral y los diálogos.",
  "dialog.themeAuto": "Automático",
  "dialog.themeAutoDesc": "Seguir el OS / navegador",
  "dialog.themeLight": "Claro",
  "dialog.themeLightDesc": "Siempre claro",
  "dialog.themeDark": "Oscuro",
  "dialog.themeDarkDesc": "Siempre oscuro",
  "dialog.themeHighContrast": "Alto contraste",
  "dialog.themeHighContrastDesc": "Accesibilidad",
  "dialog.ruler": "Regla",
  "dialog.rulerShow": "Mostrar regla alrededor de la diapositiva",
  "dialog.rulerUnitLabel": "Unidad",
  "dialog.rulerUnitDesc":
    "PowerPoint centra la diapositiva en 0 con marcas en cm. El modo píxel muestra coordenadas de pantalla desde el borde.",
  "dialog.rulerUnitCm": "Centímetros (cm)",
  "dialog.rulerUnitCmDesc": "Predeterminado de PowerPoint",
  "dialog.rulerUnitPx": "Píxeles (px)",
  "dialog.rulerUnitPxDesc": "Píxeles en pantalla",
  "dialog.language": "Idioma",
  "dialog.languageDesc":
    "Elija el idioma de la interfaz. Automático sigue su preferencia de navegador / OS.",
  "dialog.languageAuto": "Automático ({detected})",
  "dialog.about": "Acerca de",
  "dialog.aboutAppName": "Aplicación",
  "dialog.aboutEngine": "Motor",
  "dialog.aboutNpmPackage": "Frontend (npm)",
  "dialog.aboutCopyright": "Copyright",
  "dialog.aboutDeveloper": "Desarrollador",
  "dialog.aboutRepository": "Repositorio",
  "dialog.aboutPackage": "Paquete",
  "dialog.aboutVersion": "Versión",
  "dialog.aboutRendering": "Entorno",
  "dialog.aboutRenderingValue": "WebAssembly · solo navegador · sin conexión",
  "dialog.aboutLicense": "Licencia",
  "dialog.viewerSettingsAriaLabel": "Ajustes del visor",
  "phase.preparingSlide": "Preparando diapositiva {current} / {total}…",
  "phase.renderingPdf": "Renderizando página PDF {current} / {total}…",
  "phase.encodingPdf": "Codificando PDF…",
  "phase.preparingSlides": "Preparando diapositivas…",
  "status.loadedSlides": "{count} diapositivas cargadas",
  "status.nothingToPrint": "Nada que imprimir.",
  "status.nothingToExport": "Nada que exportar.",
  "status.pdfFailed": "Error al exportar PDF: {reason}",
  "status.exported": "Exportadas {count} diapositivas a PDF.",
  "status.errorPrefix": "[error:{phase}] {message}",
  "viewer.ariaLabel": "Visor PPTX",
  "viewer.noFile": "(sin archivo)",
  "viewer.slideTitle": "Diapositiva {number}",
  "viewer.empty": "Abre un archivo .pptx.",
  "viewer.openFile": "Abrir archivo",
  "phase.preparingSlideOf": "Preparando diapositiva {current} / {total}",
  "phase.preparingPdf": "Preparando PDF…",
  "phase.preparingPrint": "Preparando impresión…",
  "phase.layingOutPrintOf": "Maquetando página {current} / {total}…",
  "phase.openingPrintDialog": "Abriendo el cuadro de diálogo de impresión…",
  "phase.savingPdf": "Guardando PDF…",
  "progress.titlePrint": "Preparar impresión",
  "progress.titlePdf": "Exportar PDF",
  "viewer.loading": "Cargando…",
  "viewer.error": "⚠ {message}",
  "playground.title": "slideglance playground",
  "playground.upload": "subir:",
  "playground.samples": "muestras:",
  "playground.pickPrompt": "Elige una muestra o sube un .pptx.",
  "playground.loadingFile": "Cargando {name}…",
  "playground.failedHttp": "Error: HTTP {status}",
  "playground.fileInfo": "{name} · {bytes} bytes · {ms} ms",
  "playground.convertFailed": "Conversión fallida: {reason}",
  "shortcuts.title": "Atajos de teclado",
  "shortcuts.openTitle": "Atajos de teclado",
  "shortcuts.groupNavigation": "Navegación",
  "shortcuts.groupView": "Vista",
  "shortcuts.groupSelection": "Selección",
  "shortcuts.groupOutput": "Buscar y salida",
  "shortcuts.prevSlide": "Diapositiva anterior",
  "shortcuts.nextSlide": "Diapositiva siguiente",
  "shortcuts.firstSlide": "Primera diapositiva",
  "shortcuts.lastSlide": "Última diapositiva",
  "shortcuts.zoomIn": "Acercar",
  "shortcuts.zoomOut": "Alejar",
  "shortcuts.zoomReset": "Restablecer al 100%",
  "shortcuts.panSlide": "Mover la diapositiva mientras se mantiene",
  "shortcuts.click": "Clic",
  "shortcuts.drag": "Arrastrar",
  "shortcuts.doubleClick": "Doble clic",
  "shortcuts.selectShape": "Seleccionar forma",
  "shortcuts.toggleSelect": "Alternar selección",
  "shortcuts.rubberBand": "Selección por arrastre",
  "shortcuts.selectAll": "Seleccionar todo",
  "shortcuts.copyText": "Copiar texto seleccionado",
  "shortcuts.editText": "Modo de edición de texto",
  "shortcuts.clearSelection": "Borrar selección / salir",
  "shortcuts.toggleSearch": "Buscar en diapositivas",
  "shortcuts.print": "Imprimir",
};

const FR: Catalog = {
  _locale: "fr",
  "common.close": "Fermer",
  "common.cancel": "Annuler",
  "common.loading": "Chargement…",
  "common.ready": "Prêt.",
  "common.bytes": "{count} octets",
  "nav.firstSlide": "Première diapositive",
  "nav.previousSlide": "Précédente (←)",
  "nav.nextSlide": "Suivante (→)",
  "nav.lastSlide": "Dernière diapositive",
  "nav.slideCounter": "{current} / {total}",
  "nav.slideCounterEmpty": "—",
  "search.button": "Rechercher",
  "search.placeholder": "Rechercher dans les diapositives…",
  "search.title": "Rechercher (Cmd/Ctrl+F)",
  "search.empty": "Rechercher",
  "search.noMatches": "Aucun résultat.",
  "search.typeToSearch": "Tapez pour rechercher.",
  "output.print": "Imprimer",
  "output.printTitle": "Imprimer (Cmd/Ctrl+P)",
  "output.pdf": "PDF",
  "output.pdfTitle": "Exporter en PDF",
  "output.slideshow": "Diaporama",
  "output.slideshowTitle": "Démarrer le diaporama (F5)",
  "output.gateLoadFirst": "Chargez un fichier d'abord",
  "output.gatePreparing":
    "Préparation des diapositives — {current} / {total} prêtes",
  "settings.title": "Paramètres",
  "settings.openTitle": "Paramètres",
  "file.open": "Ouvrir",
  "render.label": "Rendu du texte :",
  "render.text": "Texte vectoriel",
  "render.path": "Contour de tracé",
  "render.auto": "Auto",
  "status.toggleNotes": "Basculer le panneau notes",
  "status.resizeSidebar": "Glisser pour redimensionner la barre des miniatures",
  "status.normalView": "Vue normale (diapositive + miniatures)",
  "status.gridView": "Trieuse de diapositives",
  "status.zoomOut": "Zoom arrière",
  "status.zoomIn": "Zoom avant",
  "status.zoomReset": "Réinitialiser à 100 %",
  "status.fitWindow": "Ajuster à la fenêtre",
  "status.zoom": "Zoom",
  "status.slideOf": "diapositive {current} / {total}",
  "status.slideEmpty": "—",
  "status.selectionFontLabel": "Police :",
  "status.selectionFontMultiple": "{first} + {extra} autres",
  "status.selectionFontTitle":
    "Police(s) utilisée(s) par la sélection : {fonts}",
  "fontUsage.title": "Correspondance des polices",
  "fontUsage.close": "Fermer",
  "fontUsage.headerRequested": "Police d'origine",
  "fontUsage.headerEffective": "Rendue comme",
  "fontUsage.systemFallback": "police système",
  "fontUsage.allMatched": "Toutes les polices d'origine sont disponibles",
  "fontUsage.substituteCount": "{count} police(s) substituée(s)",
  "notes.heading": "Notes — diapositive {current}",
  "notes.headingWithSection": "Notes — diapositive {current} · {section}",
  "notes.empty": "Aucune note pour cette diapositive.",
  "notes.standaloneHeading": "Notes — diapositive {current}",
  "notes.standaloneEmpty":
    "Aucune note de présentateur pour cette diapositive.",
  "notes.layoutLabel": "disposition : {value}",
  "notes.sectionLabel": "section : {value}",
  "notes.noSlide": "Aucune diapositive chargée.",
  "section.empty": "aucune section",
  "dialog.title": "Paramètres",
  "dialog.appearance": "Apparence",
  "dialog.theme": "Thème",
  "dialog.themeDesc":
    "Appliqué à l'ensemble du visualiseur, y compris la scène, le ruban, la barre latérale et les boîtes de dialogue.",
  "dialog.themeAuto": "Auto",
  "dialog.themeAutoDesc": "Suivre OS / navigateur",
  "dialog.themeLight": "Clair",
  "dialog.themeLightDesc": "Toujours clair",
  "dialog.themeDark": "Sombre",
  "dialog.themeDarkDesc": "Toujours sombre",
  "dialog.themeHighContrast": "Contraste élevé",
  "dialog.themeHighContrastDesc": "Accessibilité",
  "dialog.ruler": "Règle",
  "dialog.rulerShow": "Afficher la règle autour de la diapositive",
  "dialog.rulerUnitLabel": "Unité",
  "dialog.rulerUnitDesc":
    "Par défaut, PowerPoint centre la diapositive sur 0 avec une graduation en cm. Le mode pixel affiche les coordonnées écran depuis le bord.",
  "dialog.rulerUnitCm": "Centimètres (cm)",
  "dialog.rulerUnitCmDesc": "Par défaut PowerPoint",
  "dialog.rulerUnitPx": "Pixels (px)",
  "dialog.rulerUnitPxDesc": "Pixels à l'écran",
  "dialog.language": "Langue",
  "dialog.languageDesc":
    "Choisissez la langue de l'interface. Auto suit votre préférence navigateur / OS.",
  "dialog.languageAuto": "Auto ({detected})",
  "dialog.about": "À propos",
  "dialog.aboutAppName": "Application",
  "dialog.aboutEngine": "Moteur",
  "dialog.aboutNpmPackage": "Frontend (npm)",
  "dialog.aboutCopyright": "Copyright",
  "dialog.aboutDeveloper": "Développeur",
  "dialog.aboutRepository": "Dépôt",
  "dialog.aboutPackage": "Paquet",
  "dialog.aboutVersion": "Version",
  "dialog.aboutRendering": "Environnement",
  "dialog.aboutRenderingValue":
    "WebAssembly · navigateur uniquement · hors-ligne",
  "dialog.aboutLicense": "Licence",
  "dialog.viewerSettingsAriaLabel": "Paramètres du visualiseur",
  "phase.preparingSlide": "Préparation de la diapositive {current} / {total}…",
  "phase.renderingPdf": "Rendu de la page PDF {current} / {total}…",
  "phase.encodingPdf": "Encodage du PDF…",
  "phase.preparingSlides": "Préparation des diapositives…",
  "status.loadedSlides": "{count} diapositives chargées",
  "status.nothingToPrint": "Rien à imprimer.",
  "status.nothingToExport": "Rien à exporter.",
  "status.pdfFailed": "Échec de l'export PDF : {reason}",
  "status.exported": "{count} diapositives exportées en PDF.",
  "status.errorPrefix": "[erreur:{phase}] {message}",
  "viewer.ariaLabel": "Visualiseur PPTX",
  "viewer.noFile": "(aucun fichier)",
  "viewer.slideTitle": "Diapositive {number}",
  "viewer.empty": "Ouvrez un fichier .pptx.",
  "viewer.openFile": "Ouvrir un fichier",
  "phase.preparingSlideOf": "Préparation de la diapositive {current} / {total}",
  "phase.preparingPdf": "Préparation du PDF…",
  "phase.preparingPrint": "Préparation de l'impression…",
  "phase.layingOutPrintOf": "Mise en page {current} / {total}…",
  "phase.openingPrintDialog": "Ouverture de la boîte de dialogue d'impression…",
  "phase.savingPdf": "Enregistrement du PDF…",
  "progress.titlePrint": "Préparer l'impression",
  "progress.titlePdf": "Exporter le PDF",
  "viewer.loading": "Chargement…",
  "viewer.error": "⚠ {message}",
  "playground.title": "slideglance playground",
  "playground.upload": "envoyer :",
  "playground.samples": "exemples :",
  "playground.pickPrompt": "Choisissez un exemple ou envoyez un .pptx.",
  "playground.loadingFile": "Chargement de {name}…",
  "playground.failedHttp": "Échec : HTTP {status}",
  "playground.fileInfo": "{name} · {bytes} octets · {ms} ms",
  "playground.convertFailed": "Échec de la conversion : {reason}",
  "shortcuts.title": "Raccourcis clavier",
  "shortcuts.openTitle": "Raccourcis clavier",
  "shortcuts.groupNavigation": "Navigation",
  "shortcuts.groupView": "Affichage",
  "shortcuts.groupSelection": "Sélection",
  "shortcuts.groupOutput": "Recherche et sortie",
  "shortcuts.prevSlide": "Diapositive précédente",
  "shortcuts.nextSlide": "Diapositive suivante",
  "shortcuts.firstSlide": "Première diapositive",
  "shortcuts.lastSlide": "Dernière diapositive",
  "shortcuts.zoomIn": "Zoom avant",
  "shortcuts.zoomOut": "Zoom arrière",
  "shortcuts.zoomReset": "Réinitialiser à 100 %",
  "shortcuts.panSlide": "Déplacer la diapositive (maintenir)",
  "shortcuts.click": "Clic",
  "shortcuts.drag": "Glisser",
  "shortcuts.doubleClick": "Double-clic",
  "shortcuts.selectShape": "Sélectionner une forme",
  "shortcuts.toggleSelect": "Basculer la sélection",
  "shortcuts.rubberBand": "Sélection par cadre",
  "shortcuts.selectAll": "Tout sélectionner",
  "shortcuts.copyText": "Copier le texte sélectionné",
  "shortcuts.editText": "Mode édition de texte",
  "shortcuts.clearSelection": "Effacer la sélection / quitter",
  "shortcuts.toggleSearch": "Rechercher dans les diapositives",
  "shortcuts.print": "Imprimer",
};

const DE: Catalog = {
  _locale: "de",
  "common.close": "Schließen",
  "common.cancel": "Abbrechen",
  "common.loading": "Lädt…",
  "common.ready": "Bereit.",
  "common.bytes": "{count} Bytes",
  "nav.firstSlide": "Erste Folie",
  "nav.previousSlide": "Zurück (←)",
  "nav.nextSlide": "Weiter (→)",
  "nav.lastSlide": "Letzte Folie",
  "nav.slideCounter": "{current} / {total}",
  "nav.slideCounterEmpty": "—",
  "search.button": "Suchen",
  "search.placeholder": "In Folien suchen…",
  "search.title": "Suchen (Cmd/Strg+F)",
  "search.empty": "Suchen",
  "search.noMatches": "Keine Treffer.",
  "search.typeToSearch": "Zum Suchen tippen.",
  "output.print": "Drucken",
  "output.printTitle": "Drucken (Cmd/Strg+P)",
  "output.pdf": "PDF",
  "output.pdfTitle": "Als PDF exportieren",
  "output.slideshow": "Bildschirmpräsentation",
  "output.slideshowTitle": "Präsentation starten (F5)",
  "output.gateLoadFirst": "Zuerst eine Datei laden",
  "output.gatePreparing":
    "Folien werden vorbereitet — {current} / {total} fertig",
  "settings.title": "Einstellungen",
  "settings.openTitle": "Einstellungen",
  "file.open": "Öffnen",
  "render.label": "Textdarstellung:",
  "render.text": "Vektortext",
  "render.path": "Pfadkontur",
  "render.auto": "Auto",
  "status.toggleNotes": "Notizenbereich umschalten",
  "status.resizeSidebar": "Miniaturleiste durch Ziehen anpassen",
  "status.normalView": "Normalansicht (Folie + Miniaturen)",
  "status.gridView": "Foliensortierung",
  "status.zoomOut": "Verkleinern",
  "status.zoomIn": "Vergrößern",
  "status.zoomReset": "Auf 100 % zurücksetzen",
  "status.fitWindow": "Folie an Fenster anpassen",
  "status.zoom": "Zoom",
  "status.slideOf": "Folie {current} / {total}",
  "status.slideEmpty": "—",
  "status.selectionFontLabel": "Schrift:",
  "status.selectionFontMultiple": "{first} +{extra} weitere",
  "status.selectionFontTitle": "Von Auswahl verwendete Schrift(en): {fonts}",
  "fontUsage.title": "Schriftartenzuordnung",
  "fontUsage.close": "Schließen",
  "fontUsage.headerRequested": "Originalschrift",
  "fontUsage.headerEffective": "Gerendert als",
  "fontUsage.systemFallback": "Systemschrift",
  "fontUsage.allMatched": "Alle Originalschriften verfügbar",
  "fontUsage.substituteCount": "{count} Schrift(en) ersetzt",
  "notes.heading": "Notizen — Folie {current}",
  "notes.headingWithSection": "Notizen — Folie {current} · {section}",
  "notes.empty": "Keine Notizen für diese Folie.",
  "notes.standaloneHeading": "Notizen — Folie {current}",
  "notes.standaloneEmpty": "Keine Sprechernotizen für diese Folie.",
  "notes.layoutLabel": "Layout: {value}",
  "notes.sectionLabel": "Abschnitt: {value}",
  "notes.noSlide": "Keine Folie geladen.",
  "section.empty": "Keine Abschnitte",
  "dialog.title": "Einstellungen",
  "dialog.appearance": "Darstellung",
  "dialog.theme": "Thema",
  "dialog.themeDesc":
    "Wird auf den gesamten Viewer angewendet — Folienbühne, Menüband, Seitenleiste und Dialoge.",
  "dialog.themeAuto": "Automatisch",
  "dialog.themeAutoDesc": "OS / Browser folgen",
  "dialog.themeLight": "Hell",
  "dialog.themeLightDesc": "Immer hell",
  "dialog.themeDark": "Dunkel",
  "dialog.themeDarkDesc": "Immer dunkel",
  "dialog.themeHighContrast": "Hoher Kontrast",
  "dialog.themeHighContrastDesc": "Barrierefreiheit",
  "dialog.ruler": "Lineal",
  "dialog.rulerShow": "Lineal um die Folie anzeigen",
  "dialog.rulerUnitLabel": "Einheit",
  "dialog.rulerUnitDesc":
    "PowerPoint zentriert die Folie standardmäßig auf 0 mit cm-Skala. Pixelmodus zeigt Bildschirmkoordinaten ab der Folienkante.",
  "dialog.rulerUnitCm": "Zentimeter (cm)",
  "dialog.rulerUnitCmDesc": "PowerPoint-Standard",
  "dialog.rulerUnitPx": "Pixel (px)",
  "dialog.rulerUnitPxDesc": "Bildschirm-Pixel",
  "dialog.language": "Sprache",
  "dialog.languageDesc":
    "Wählen Sie die Oberflächensprache. Automatisch folgt Ihrer Browser- / OS-Einstellung.",
  "dialog.languageAuto": "Automatisch ({detected})",
  "dialog.about": "Über",
  "dialog.aboutAppName": "Anwendung",
  "dialog.aboutEngine": "Engine",
  "dialog.aboutNpmPackage": "Frontend (npm)",
  "dialog.aboutCopyright": "Copyright",
  "dialog.aboutDeveloper": "Entwickler",
  "dialog.aboutRepository": "Repository",
  "dialog.aboutPackage": "Paket",
  "dialog.aboutVersion": "Version",
  "dialog.aboutRendering": "Umgebung",
  "dialog.aboutRenderingValue": "WebAssembly · nur Browser · offline-fähig",
  "dialog.aboutLicense": "Lizenz",
  "dialog.viewerSettingsAriaLabel": "Viewer-Einstellungen",
  "phase.preparingSlide": "Folie wird vorbereitet {current} / {total}…",
  "phase.renderingPdf": "PDF-Seite wird gerendert {current} / {total}…",
  "phase.encodingPdf": "PDF wird codiert…",
  "phase.preparingSlides": "Folien werden vorbereitet…",
  "status.loadedSlides": "{count} Folien geladen",
  "status.nothingToPrint": "Nichts zu drucken.",
  "status.nothingToExport": "Nichts zu exportieren.",
  "status.pdfFailed": "PDF-Export fehlgeschlagen: {reason}",
  "status.exported": "{count} Folien als PDF exportiert.",
  "status.errorPrefix": "[Fehler:{phase}] {message}",
  "viewer.ariaLabel": "PPTX-Viewer",
  "viewer.noFile": "(keine Datei)",
  "viewer.slideTitle": "Folie {number}",
  "viewer.empty": "Eine .pptx-Datei öffnen.",
  "viewer.openFile": "Datei öffnen",
  "phase.preparingSlideOf": "Folie {current} / {total} wird vorbereitet",
  "phase.preparingPdf": "PDF wird vorbereitet…",
  "phase.preparingPrint": "Druck wird vorbereitet…",
  "phase.layingOutPrintOf": "Seite {current} / {total} wird gesetzt…",
  "phase.openingPrintDialog": "Druckdialog wird geöffnet…",
  "phase.savingPdf": "PDF wird gespeichert…",
  "progress.titlePrint": "Druck vorbereiten",
  "progress.titlePdf": "PDF exportieren",
  "viewer.loading": "Lädt…",
  "viewer.error": "⚠ {message}",
  "playground.title": "slideglance Playground",
  "playground.upload": "Hochladen:",
  "playground.samples": "Beispiele:",
  "playground.pickPrompt": "Beispiel wählen oder .pptx hochladen.",
  "playground.loadingFile": "{name} wird geladen…",
  "playground.failedHttp": "Fehler: HTTP {status}",
  "playground.fileInfo": "{name} · {bytes} Bytes · {ms} ms",
  "playground.convertFailed": "Konvertierung fehlgeschlagen: {reason}",
  "shortcuts.title": "Tastenkürzel",
  "shortcuts.openTitle": "Tastenkürzel",
  "shortcuts.groupNavigation": "Navigation",
  "shortcuts.groupView": "Ansicht",
  "shortcuts.groupSelection": "Auswahl",
  "shortcuts.groupOutput": "Suche & Ausgabe",
  "shortcuts.prevSlide": "Vorherige Folie",
  "shortcuts.nextSlide": "Nächste Folie",
  "shortcuts.firstSlide": "Erste Folie",
  "shortcuts.lastSlide": "Letzte Folie",
  "shortcuts.zoomIn": "Vergrößern",
  "shortcuts.zoomOut": "Verkleinern",
  "shortcuts.zoomReset": "Auf 100 % zurücksetzen",
  "shortcuts.panSlide": "Folie verschieben (halten)",
  "shortcuts.click": "Klick",
  "shortcuts.drag": "Ziehen",
  "shortcuts.doubleClick": "Doppelklick",
  "shortcuts.selectShape": "Form auswählen",
  "shortcuts.toggleSelect": "Auswahl umschalten",
  "shortcuts.rubberBand": "Lasso-Auswahl",
  "shortcuts.selectAll": "Alles auswählen",
  "shortcuts.copyText": "Ausgewählten Text kopieren",
  "shortcuts.editText": "Textbearbeitungsmodus",
  "shortcuts.clearSelection": "Auswahl aufheben / beenden",
  "shortcuts.toggleSearch": "In Folien suchen",
  "shortcuts.print": "Drucken",
};

const CATALOGS: Record<ResolvedLocale, Catalog> = {
  en: EN,
  ko: KO,
  ja: JA,
  "zh-CN": ZH_CN,
  "zh-TW": ZH_TW,
  es: ES,
  fr: FR,
  de: DE,
};

/**
 * Map a raw `navigator.language(s)` or POSIX locale tag to a
 * supported locale. Region/script subtags are honoured for Chinese
 * (`zh-CN` vs `zh-TW`) and stripped for everything else.
 */
function normalizeTag(raw: string): ResolvedLocale | null {
  if (!raw) return null;
  // POSIX `ko_KR.UTF-8` → `ko-KR`.
  const cleaned = raw.replace(/[._].*$/, "").replace(/_/g, "-");
  const lower = cleaned.toLowerCase();
  // Chinese region / script — be explicit so Simplified vs Traditional
  // doesn't collapse onto whichever happens to come first.
  if (lower.startsWith("zh")) {
    if (
      lower.includes("tw") ||
      lower.includes("hk") ||
      lower.includes("mo") ||
      lower.includes("hant")
    ) {
      return "zh-TW";
    }
    return "zh-CN";
  }
  const primary = lower.split("-")[0] as ResolvedLocale;
  if ((SUPPORTED_LOCALES as readonly string[]).includes(primary)) {
    return primary;
  }
  return null;
}

/**
 * Read the host's preferred locale list. In the browser this is
 * `navigator.languages`; in Node we walk the standard POSIX
 * environment variables. Returns the first entry we recognise, or
 * `"en"` as the universal fallback.
 */
export function detectLocale(): ResolvedLocale {
  const candidates: string[] = [];
  if (typeof navigator !== "undefined") {
    const langs = (navigator as Navigator & { languages?: readonly string[] })
      .languages;
    if (Array.isArray(langs)) candidates.push(...langs);
    if (typeof navigator.language === "string")
      candidates.push(navigator.language);
  }
  // POSIX fallback (Node, Vitest happy-dom without navigator.language).
  const env = (
    globalThis as { process?: { env?: Record<string, string | undefined> } }
  ).process?.env;
  if (env) {
    for (const key of ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] as const) {
      const v = env[key];
      if (typeof v === "string" && v.length > 0) candidates.push(v);
    }
  }
  for (const tag of candidates) {
    const m = normalizeTag(tag);
    if (m) return m;
  }
  return "en";
}

type Listener = (locale: ResolvedLocale) => void;
const listeners = new Set<Listener>();

let forced: ResolvedLocale | null = null;
let detectedCache: ResolvedLocale | null = null;

function detected(): ResolvedLocale {
  if (detectedCache) return detectedCache;
  detectedCache = detectLocale();
  return detectedCache;
}

/** Reset the detection cache — primarily for tests that mutate
 *  `navigator.language` between cases. */
export function resetLocaleCache(): void {
  detectedCache = null;
}

/** Currently effective locale (auto-detected unless forced). */
export function getActiveLocale(): ResolvedLocale {
  return forced ?? detected();
}

/** Auto-detected locale, regardless of the active override. */
export function getDetectedLocale(): ResolvedLocale {
  return detected();
}

/**
 * Apply a forced locale, or pass `"auto"` / `null` to clear the
 * override and revert to auto-detection. Subscribers fire whenever
 * the resolved locale actually changes.
 */
export function setLocale(locale: Locale | null): void {
  const previous = getActiveLocale();
  if (locale === null || locale === "auto") {
    forced = null;
  } else {
    forced = locale;
  }
  const next = getActiveLocale();
  if (next !== previous) {
    for (const listener of listeners) {
      try {
        listener(next);
      } catch {
        /* one bad listener should not break the others. */
      }
    }
  }
}

/** Subscribe to active-locale changes. Returns a teardown. */
export function subscribeLocale(cb: Listener): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

/**
 * Look up a translated message and substitute `{token}` placeholders.
 * Falls back to the English catalog when the active locale is missing
 * the key, and finally to the key itself when even English doesn't
 * have it (which only happens for keys added without a catalog
 * update — surface them visibly so they get noticed in review).
 */
export function t(
  key: MessageKey,
  params?: Record<string, string | number>,
): string {
  const active = getActiveLocale();
  const cat = CATALOGS[active];
  let template = cat[key];
  if (template === undefined && active !== "en") template = EN[key];
  if (template === undefined) return key;
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_match, name: string) => {
    const v = params[name];
    return v === undefined ? `{${name}}` : String(v);
  });
}
