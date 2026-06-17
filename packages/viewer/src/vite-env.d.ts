/// <reference types="vite/client" />

// Vite `define` injects these at build time from the viewer's
// package.json (see vite.config.ts). They are the single source of
// truth for the displayed product name and version — never hardcode
// a separate value.
declare const __PPTX_VIEWER_NAME__: string;
declare const __PPTX_VIEWER_VERSION__: string;

declare module "*?worker" {
  const WorkerCtor: { new (): Worker };
  export default WorkerCtor;
}

declare module "*?worker&url" {
  const url: string;
  export default url;
}
