#!/usr/bin/env bash
set -euo pipefail

output_type="${1:-${BLUESHELL_OUTPUT_TYPE:-exe}}"
config="${2:-${BLUESHELL_BUILD_CONFIG:-build-config.json}}"
rm -rf build dist
mkdir -p dist
cmake -S . -B build -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_TOOLCHAIN_FILE=cmake/mingw64.cmake \
  -DBLUESHELL_CONFIG="$(realpath "$config")"

case "$output_type" in
  exe) cmake --build build --target stage0_exe; cp build/stage0.exe dist/stage0.exe ;;
  service_exe) cmake --build build --target stage0_service; cp build/payload-service.exe dist/payload-service.exe ;;
  dll) cmake --build build --target stage0_dll; cp build/stage0.dll dist/stage0.dll ;;
  raw|shellcode) cmake --build build --target stage0_raw; cp build/stage0.raw dist/payload.bin ;;
  *) echo "unsupported output type: $output_type" >&2; exit 2 ;;
esac
