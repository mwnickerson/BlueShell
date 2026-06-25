# Testing

## Local checks

Run:

```bash
make validate
```

This performs Python syntax/layout validation and compiles/tests both native
agent trees when the required local toolchains exist.

The authoritative build environment is the payload container because it
contains the Windows cross-compilers and binary conversion tools.

## Mythic acceptance matrix

For each of `blueshell_stage0` and `blueshell_stage1`:

1. Build against `http`, `httpx`, `smb`, and `tcp`.
2. Generate EXE, DLL, service, and shellcode output.
3. Verify checkin and callback UUID transition.
4. Exercise sleep/jitter, execution, upload, download, and fingerprint.
5. Run a benign COFF fixture and verify output/error handling.
6. Pass traffic through SOCKS and reverse port forwarding.
7. Confirm clean exit and proxy connection teardown.

For stage 0, additionally retrieve and start stage 1 and confirm that stage 0
continues to work when promotion is not used.

### COFF compatibility

Stage 1 accepts Windows AMD64 COFF objects, resolves `LIBRARY$Function`
imports, applies common AMD64 relocations, and captures `BeaconOutput` plus the
common `BeaconPrintf` subset. Validate representative BOFs that exercise:

- direct and `__imp_` dynamic function imports;
- `ADDR64`, `ADDR32`, `ADDR32NB`, and `REL32` relocation variants;
- entrypoint selection and packed argument delivery;
- output, missing-symbol, and malformed-object behavior.

The current `BeaconPrintf` bridge supports common format specifiers and two
variadic values. Additional Beacon APIs should be registered as internal
symbols before claiming broad third-party BOF compatibility.

## Artifact review

- Compare builds with two polymorphism seeds; hashes must differ.
- Run `strings` and reject target-side framework, repository, author, and debug
  identifiers.
- Verify release artifacts have no debug symbols.
- Record size for every output type and fail regressions above the release
  budget chosen after the first working baseline.
