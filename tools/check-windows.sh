#!/bin/sh
# RUNNER-WIN: cross-check the runner and machine-stats for Windows from Linux
# (cargo check, no linking). Usage: sh tools/check-windows.sh [cargo args...]
# Runs in a rust container with mingw so C build scripts (ring) compile; the registry
# and target dir live in named volumes between runs.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
dir=${CHECK_DIR:-runner}
docker run --rm -i \
  -e CARGO_TARGET_DIR=/cache/target -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" \
  -v kompanion-cargo-registry-bookworm:/usr/local/cargo/registry \
  -v kompanion-target-win:/cache/target \
  -v "$root":/src:ro -w "/src/$dir" rust:1-bookworm sh -eu -c '
    apt-get update -qq >/dev/null && apt-get install -y -qq gcc-mingw-w64-x86-64 >/dev/null
    rustup target add x86_64-pc-windows-gnu >/dev/null 2>&1
    cargo check --locked --target x86_64-pc-windows-gnu --all-targets "$@"
  ' sh "$@"
