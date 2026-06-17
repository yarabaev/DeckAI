# playground-samples

Curated PPTX sample decks.

Each deck is structured the same way as the [reference deck](../builder-reference): every page is
composed from a slide **master**, a small set of **style sheets**, a handful of reusable
**templates**, and chapter files that drop content into those templates.

## Decks

| Slug        | Theme                      | Canvas            | Highlights                                                        |
| ----------- | -------------------------- | ----------------- | ----------------------------------------------------------------- |
| `pitch`     | Startup pitch (dark)       | 16:9 · 1280 × 720 | Gradient cover, accent stripe master, metric cards, quote slide.  |
| `editorial` | Editorial report (serif)   | A4 · 793 × 1122   | Magazine masthead, two-column body, drop cap, pull quote, figure. |
| `tech-spec` | Technical spec (corporate) | 16:9 · 1280 × 720 | Sectioned eyebrow master, architecture diagram, comparison grid.  |
| `workshop`  | Workshop / tutorial        | 16:9 · 1280 × 720 | Step ribbon master, code-block template, callout, exercise card.  |

## Layout

```
playground-samples/
├── build.ts                   # compiles every deck, copies .pptx into web-playground/public/samples/
├── package.json
├── tsconfig.json
└── <slug>/
    ├── main.sgx               # document root, imports styles → master → templates → chapters
    ├── styles/
    │   ├── colors.xml
    │   └── typography.xml
    ├── templates/
    │   ├── master.xml         # <Master> definitions
    │   ├── page.xml           # generic page wrappers
    │   └── components.xml     # reusable building blocks (cards, callouts, …)
    └── chapters/
        └── NN-<topic>.xml     # <Slide> / <Use template=…> entries
```

## Build

```sh
pnpm --filter @slideglance/playground-samples run build
```

Outputs to `apps/web-playground/public/samples/` and overwrites any prior files.

## Adding a new theme

1. Copy any existing deck folder under a new slug.
2. Append it to the `DECKS` array in `build.ts`.
3. Rebuild and append the resulting file to the `SAMPLES` list in
   `apps/web-playground/src/main.tsx`.
