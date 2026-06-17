#!/usr/bin/env bash
#
# Build the @slideglance/* wasm artefacts for every consumer target.
#
# Why this script exists
# ----------------------
# wasm-pack writes to a single `--out-dir`. Each `@slideglance/*` npm
# package ships THREE target builds — `dist/bundler/`, `dist/web/`,
# `dist/node/` — because consumers pick whichever `exports.*` subpath
# matches their tooling. Forgetting to refresh ALL three after a Rust
# change leaves stale wasm in whichever target the consumer happens to
# import (most commonly `dist/bundler/`, which the playground imports
# through Vite).
#
# Two crates ship through this script:
#   - `slideglance-wasm`         → `@slideglance/core`    (full PPTX pipeline)
#   - `slideglance-measure-wasm` → `@slideglance/measure` (text measurement only)
#
# Wired as a `prebuild` hook on every JS package that depends on either
# wasm bundle, so a stale .wasm bundled into a downstream app cannot
# happen by accident.
#
# Up-to-date detection
# --------------------
# The hook runs on every JS build, but the heavy `wasm-pack` work is
# only done when something under `crates/` (or `Cargo.toml` /
# `Cargo.lock` / this script itself / the cached profile) is newer
# than the most recent wasm artefact. On a no-op rebuild the script
# returns in well under 100 ms — the cost of one `find` walk over
# `crates/` minus its `target/` subtree.
#
# Usage
# -----
#   ./scripts/build-wasm.sh             # release builds (default)
#   PROFILE=dev ./scripts/build-wasm.sh # debug builds (faster, larger)
#   FORCE=1 ./scripts/build-wasm.sh     # always rebuild, ignore cache

set -euo pipefail

PROFILE="${PROFILE:-release}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT_PATH="${BASH_SOURCE[0]}"

# Defense-in-depth serialization
# ------------------------------
# CI workflows build wasm artefacts up front (`bash scripts/build-wasm.sh`
# as an explicit step) so the per-package `prebuild` hooks on viewer /
# chrome-extension / web-playground collapse to cache-hit no-ops without
# contending here. This lock exists for edge cases the primary path
# does not cover — chiefly two dev terminals invoking `pnpm dev` and
# `pnpm build` simultaneously, or hand-running `bash scripts/build-wasm.sh`
# while a `pnpm --filter X build` is already in flight.
# Without it, two parallel processes race on the wasm-pack global cache
# (`~/.wasm-pack/...`) and on `wasm-opt`, producing platform-specific
# failures:
#   - macOS:   `wasm-bindgen is not executable` (chmod race)
#   - Linux:   `invalid type: sequence, expected a string` (cache file
#              half-written when serde reads it)
#   - Windows: `wasm-opt`: file lock (os error 32)
# `mkdir` is atomic on POSIX and on NTFS via Git Bash, so it works as
# a cheap cross-platform lock without extra binaries (`flock` is not
# guaranteed on Windows runners). The cache check inside `build_one`
# is then a no-op for the second arrival.
LOCK_DIR="${ROOT}/target/.build-wasm.lock"
acquire_lock() {
  mkdir -p "${ROOT}/target"
  local waited=0
  while ! mkdir "${LOCK_DIR}" 2>/dev/null; do
    if [[ ${waited} -ge 900 ]]; then
      echo "[build-wasm] timeout waiting for lock at ${LOCK_DIR}" >&2
      exit 1
    fi
    if [[ ${waited} -eq 0 ]]; then
      echo "[build-wasm] another build-wasm is running, waiting on ${LOCK_DIR}"
    fi
    sleep 1
    waited=$((waited + 1))
  done
  trap 'rmdir "${LOCK_DIR}" 2>/dev/null || true' EXIT INT TERM
}

# (crate_dir, out_base, label) triples — one per wasm-pack invocation
# group. Adding a new wasm-bundled crate is a single-line change here.
CRATES=(
  "slideglance-wasm:core:slideglance_wasm"
  "slideglance-measure-wasm:measure:slideglance_measure_wasm"
)

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "error: wasm-pack not found in PATH" >&2
  echo "       install via 'cargo install wasm-pack' or" >&2
  echo "       'curl https://rustwasm.github.io/wasm-pack/installer/init.sh | sh'" >&2
  exit 1
fi

PROFILE_FLAG=""
if [[ "${PROFILE}" == "dev" ]]; then
  PROFILE_FLAG="--dev"
elif [[ "${PROFILE}" != "release" ]]; then
  echo "error: PROFILE must be 'release' or 'dev', got '${PROFILE}'" >&2
  exit 1
fi

# Stamp file (per package) records the profile of the last successful
# build. Profile changes (`release` ↔ `dev`) must always trigger a
# rebuild because wasm-pack overwrites the same out-dir with binaries
# that have very different size and debuginfo characteristics.
needs_rebuild() {
  local out_base="$1"
  local artefact="$2" # one wasm path used as the up-to-date proxy
  local stamp="${out_base}/.build-wasm.profile"
  local last_profile=""
  if [[ -f "${stamp}" ]]; then
    last_profile=$(cat "${stamp}")
  fi
  if [[ "${FORCE:-0}" == "1" || "${last_profile}" != "${PROFILE}" || ! -f "${artefact}" ]]; then
    return 0
  fi
  # find returns the first source file newer than the artefact and
  # exits — empty result means everything is up to date. Narrow the
  # walk to files that can actually change the compiled wasm output:
  # `.rs` sources and per-crate `Cargo.toml` manifests. Documentation
  # (`docs/**`, `README.md`), test fixtures, build snapshots, etc.
  # don't affect the binary, and including them caused spurious
  # rebuilds in CI where `git checkout` stamps every file with the
  # current time and the wasm dist (restored from a content-addressed
  # cache) ends up older than unrelated docs.
  local newer
  newer=$(find "${ROOT}/crates" \
            \! -path "${ROOT}/crates/target*" \
            -type f \
            \( -name '*.rs' -o -name 'Cargo.toml' \) \
            -newer "${artefact}" -print -quit 2>/dev/null || true)
  if [[ -z "${newer}" ]]; then
    for extra in "${ROOT}/Cargo.toml" "${ROOT}/Cargo.lock" "${ROOT}/rust-toolchain.toml" "${SCRIPT_PATH}"; do
      if [[ -f "${extra}" && "${extra}" -nt "${artefact}" ]]; then
        newer="${extra}"
        break
      fi
    done
  fi
  if [[ -n "${newer}" ]]; then
    echo "[build-wasm] source changed: ${newer}"
    return 0
  fi
  return 1
}

build_one() {
  local crate_name="$1"
  local pkg_name="$2"
  local label="$3"
  local crate_dir="${ROOT}/crates/${crate_name}"
  local out_base="${ROOT}/packages/${pkg_name}/dist"
  local proxy_artefact="${out_base}/bundler/${label}_bg.wasm"

  if ! needs_rebuild "${out_base}" "${proxy_artefact}"; then
    echo "[build-wasm] ${crate_name} -> ${pkg_name}: up to date (profile=${PROFILE}). Set FORCE=1 to override."
    return 0
  fi

  echo "[build-wasm] crate       = ${crate_dir}"
  echo "[build-wasm] out base    = ${out_base}"
  echo "[build-wasm] profile     = ${PROFILE}"

  for target in bundler web nodejs; do
    case "${target}" in
      bundler) outdir="${out_base}/bundler" ;;
      web)     outdir="${out_base}/web" ;;
      nodejs)  outdir="${out_base}/node" ;;
    esac
    echo
    echo "[build-wasm] >>> ${crate_name} target=${target} -> ${outdir}"
    wasm-pack build \
      "${crate_dir}" \
      --target "${target}" \
      --out-dir "${outdir}" \
      ${PROFILE_FLAG}
  done

  mkdir -p "${out_base}"
  echo "${PROFILE}" > "${out_base}/.build-wasm.profile"
}

echo "[build-wasm] root        = ${ROOT}"
acquire_lock
for entry in "${CRATES[@]}"; do
  IFS=':' read -r crate_name pkg_name label <<<"${entry}"
  build_one "${crate_name}" "${pkg_name}" "${label}"
done

echo
echo "[build-wasm] all targets built successfully."
