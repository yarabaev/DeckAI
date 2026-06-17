---
title: SlideGlance Documentation
kind: navigation
last_verified_commit: 0000000000000000000000000000000000000000
source_files:
  - docs/en/
---

# SlideGlance Documentation

> Language-agnostic gateway. The docs themselves live under each
> language directory.

## Languages

- [English](./en/index.md)

> Future translations land in sibling directories (`ko/`, `ja/`, …).
> Filenames mirror `en/` exactly; the `lang:` frontmatter field
> matches the directory.

## How the docs are organised

```
docs/
├── INDEX.md            # ← you are here
└── en/
    ├── index.md        # workspace overview
    ├── architecture.md
    ├── distribution.md
    ├── fonts.md
    ├── crates.md       # → per-crate docs
    ├── packages.md     # → per-package docs
    └── apps.md         # → per-app docs
```

Each `crates/<name>/`, `packages/<name>/`, and `apps/<name>/` also
carries its own `docs/en/{index,reference,guides}.md` triple.
Navigation flows top-down from the workspace overview into each
member.
