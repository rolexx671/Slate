#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
if [[ "$(uname -m)" != arm64 || "$(uname -s)" != Darwin ]]; then
  echo 'Этот сценарий предназначен для macOS с Apple Silicon.' >&2
  exit 1
fi
if [[ ! -f src-tauri/libpdfium.dylib ]]; then
  task_tmp="$(mktemp -d)"
  trap 'rm -rf "$task_tmp"' EXIT
  curl --fail --location --retry 3 'https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7891/pdfium-mac-arm64.tgz' -o "$task_tmp/pdfium.tgz"
  tar -xzf "$task_tmp/pdfium.tgz" -C "$task_tmp"
  cp "$task_tmp/lib/libpdfium.dylib" src-tauri/libpdfium.dylib
fi
printf '%s\n' 'f71102b96ff0c56728b3eaa9a26beab9fec3fb0c2e73319c0f3bfe2f32222948  src-tauri/libpdfium.dylib' | shasum -a 256 -c -
cargo tauri build --bundles app
printf '%s\n' 'Готово: src-tauri/target/release/bundle/macos/Slate RU.app'
