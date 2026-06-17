# syntax=docker/dockerfile:1
FROM node:22-slim

# Install system packages (BuildKit cache mounts).
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    libreoffice-impress \
    imagemagick \
    ghostscript \
    fonts-noto-cjk \
    fonts-noto-cjk-extra \
    && rm -rf /usr/share/doc/* \
    && rm -rf /usr/share/man/* \
    && sed -i 's/rights="none" pattern="PDF"/rights="read|write" pattern="PDF"/' /etc/ImageMagick-6/policy.xml

# pnpm setup (version pinned via root package.json `packageManager`).
WORKDIR /app
COPY package.json ./
RUN corepack enable && corepack install

# pnpm install (BuildKit store cache). `.npmrc` is optional — the
# repo does not commit one. Add it back here if you introduce a
# project-wide registry override or auth setup.
COPY pnpm-lock.yaml pnpm-workspace.yaml ./
COPY packages/core/package.json packages/core/
COPY packages/measure/package.json packages/measure/
COPY packages/viewer/package.json packages/viewer/
COPY packages/builder/package.json packages/builder/
COPY apps/vscode-extension/package.json apps/vscode-extension/
COPY examples/builder-reference/package.json examples/builder-reference/
# Local pnpm `patchedDependencies` need their patch files in place
# before `pnpm install` resolves the lockfile.
COPY patches/ patches/
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile

# Copy application sources.
COPY . .

WORKDIR /app/packages/builder
CMD ["pnpm", "run", "vrt"]
