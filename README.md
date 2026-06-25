# BlueShell

BlueShell is a Windows-first Mythic agent family with two payload types:

- **`blueshell_stage0`** — a compact, PIC-oriented C/C++ agent that can operate
  independently or promote to stage 1.
- **`blueshell_stage1`** — a modular Rust agent for longer-running operations.

Both payloads are designed for Mythic 3.x and expose `http`, `httpx`, `smb`, and
`tcp` C2 profile compatibility. Windows x64 is the initial supported target.

## Repository layout

```text
Payload_Type/blueshell/
  Dockerfile
  main.py
  blueshell/
    mythic/agent_functions/  # Payload and command definitions
    agent_code/
      stage0/                # C/C++ implementation
      stage1/                # Rust implementation
```

## Install

From the Mythic directory:

```bash
sudo ./mythic-cli install github https://github.com/<owner>/BlueShell
sudo ./mythic-cli start blueshell
sudo ./mythic-cli logs blueshell
```

For a local checkout, install the repository path using the corresponding
`mythic-cli install folder` workflow supported by your Mythic release.

Install the selected C2 profiles separately from
`github.com/MythicC2Profiles`: `http`, `httpx`, `smb`, and `tcp`.

## Development

```bash
make validate
```

The payload builder compiles inside the BlueShell Mythic container. The
container therefore includes Rust, MinGW-w64, CMake, LLVM, and binary
conversion tooling; Docker-in-Docker is not used.

Release artifacts intentionally omit debug output and symbols. Build-time
configuration controls the selected command modules, output format, transport,
and polymorphism seed.

## Output formats

- Windows executable
- Windows DLL
- Windows service executable
- Raw shellcode

Service and DLL outputs are generated through the payload builder's wrapper
pipeline from the same stage-specific source state.

## Status

This repository currently provides the installable payload container, native
build pipelines, Stage 1 protocol/runtime foundation, and Stage 0 compact
runtime foundation. Windows cross-builds must be performed in the supplied
container. Before operational use, complete the acceptance matrix in
`docs/TESTING.md`; in particular, Stage 0's compact framing layer still needs
the final Mythic envelope adapter and the service output needs SCM lifecycle
validation.

Linux is a planned follow-up and is not advertised as supported by the current
payload definitions.
