# stage0

Compact Windows x64 bootstrap agent. The internal protocol is binary and
allocation is centralized so the same core can be embedded by a later PIC
converter.

```sh
cmake -S . -B build -G Ninja \
  -DCMAKE_TOOLCHAIN_FILE=cmake/mingw64.cmake \
  -DCMAKE_BUILD_TYPE=Release -DSTAGE0_TRANSPORT=2
cmake --build build
```

Transport selectors: `1` HTTP, `2` HTTPS, `3` TCP, `4` SMB named pipe.
Outputs are `stage0.exe`, `stage0.dll`, and, when objcopy is available,
`stage0.raw` (the executable `.text` section for a downstream PIC converter).

