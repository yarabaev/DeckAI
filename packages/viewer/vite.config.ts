import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const externals = [
  "react",
  "react-dom",
  "react/jsx-runtime",
  "react/jsx-dev-runtime",
  "@phosphor-icons/react",
  "@slideglance/core",
  "@slideglance/core/bundler",
];

const pkg = JSON.parse(
  readFileSync(
    fileURLToPath(new URL("./package.json", import.meta.url)),
    "utf-8",
  ),
) as { name: string; version: string };

export default defineConfig({
  plugins: [react()],
  define: {
    // Inlined at build time so consumers can show the shipped viewer
    // name + version without a separate fetch.
    __PPTX_VIEWER_NAME__: JSON.stringify(pkg.name),
    __PPTX_VIEWER_VERSION__: JSON.stringify(pkg.version),
  },
  build: {
    target: "es2022",
    lib: {
      // Multi-entry: emit `pptx-viewer.js` (React shell) and
      // `pptx-worker.js` (Web Worker) side by side.
      entry: {
        "pptx-viewer": "src/index.ts",
        "pptx-worker": "src/pptx-worker.ts",
      },
      formats: ["es"],
    },
    rollupOptions: {
      external: externals,
      output: {
        entryFileNames: "[name].js",
      },
    },
    sourcemap: true,
    minify: false,
  },
  worker: {
    format: "es",
    rollupOptions: {
      external: externals,
    },
  },
  test: {
    environment: "happy-dom",
    globals: true,
    include: ["test/**/*.test.ts", "test/**/*.test.tsx"],
    exclude: ["node_modules/**", "dist/**"],
  },
});
