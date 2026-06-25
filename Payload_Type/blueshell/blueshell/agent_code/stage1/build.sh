#!/usr/bin/env bash
set -euo pipefail

output_type="${1:-${BLUESHELL_OUTPUT_TYPE:-exe}}"
config="${2:-${BLUESHELL_BUILD_CONFIG:-build-config.json}}"
target="${BLUESHELL_TARGET:-x86_64-pc-windows-gnu}"
export BLUESHELL_BUILD_CONFIG="$(realpath "$config")"
export CARGO_HOME="${CARGO_HOME:-/root/.cargo}"

rm -rf dist
mkdir -p dist
features=()
if [[ "${BLUESHELL_DEBUG:-0}" == "1" ]]; then
  features=(--features diagnostics)
fi
cargo_args=(build --release --locked --target "$target")
if [[ -f "$CARGO_HOME/config.toml" && -d /vendor ]]; then
  cargo_args+=(
    --offline
    --config 'source.crates-io.replace-with="vendored-sources"'
    --config 'source.vendored-sources.directory="/vendor"'
  )
else
  echo "vendored Rust dependencies unavailable; attempting online Cargo resolution" >&2
fi
cargo "${cargo_args[@]}" "${features[@]}"
release="target/$target/release"

case "$output_type" in
  exe)
    cp "$release/stage1.exe" dist/stage1.exe
    ;;
  service_exe)
    cp "$release/stage1.exe" dist/payload-service.exe
    ;;
  dll)
    cp "$release/stage1.dll" dist/stage1.dll
    ;;
  raw|shellcode)
    objcopy="${OBJCOPY:-x86_64-w64-mingw32-objcopy}"
    "$objcopy" -O binary --only-section=.text "$release/stage1.exe" dist/payload.bin
    ;;
  *)
    echo "unsupported output type: $output_type" >&2
    exit 2
    ;;
esac
