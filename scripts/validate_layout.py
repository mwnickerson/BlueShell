#!/usr/bin/env python3
"""Validate the installable Mythic external-agent layout."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SERVICE = ROOT / "Payload_Type" / "blueshell"

REQUIRED = (
    ROOT / "config.json",
    ROOT / "agent_capabilities.json",
    SERVICE / "Dockerfile",
    SERVICE / "main.py",
    SERVICE / "blueshell" / "mythic" / "agent_functions" / "__init__.py",
    SERVICE / "blueshell" / "agent_code" / "stage0" / "CMakeLists.txt",
    SERVICE / "blueshell" / "agent_code" / "stage1" / "Cargo.toml",
)


def main() -> None:
    missing = [str(path.relative_to(ROOT)) for path in REQUIRED if not path.exists()]
    if missing:
        raise SystemExit("missing required paths:\n- " + "\n- ".join(missing))

    capabilities = json.loads((ROOT / "agent_capabilities.json").read_text())
    required_c2 = {"http", "httpx", "smb", "tcp"}
    actual_c2 = set(capabilities["c2"])
    if actual_c2 != required_c2:
        raise SystemExit(f"unexpected C2 profile set: {sorted(actual_c2)}")

    outputs = set(capabilities["payload_output"])
    required_outputs = {"exe", "dll", "service", "shellcode"}
    if outputs != required_outputs:
        raise SystemExit(f"unexpected output set: {sorted(outputs)}")

    print("BlueShell external-agent layout is valid")


if __name__ == "__main__":
    main()
