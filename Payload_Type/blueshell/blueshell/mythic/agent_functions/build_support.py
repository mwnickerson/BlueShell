"""Build plumbing shared by the two BlueShell payload definitions."""

from __future__ import annotations

import base64
import json
import os
import shutil
import subprocess
import tempfile
from urllib.parse import urlparse
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

OUTPUT_EXTENSIONS = {
    "shellcode": ".bin",
    "raw": ".bin",
    "exe": ".exe",
    "service_exe": ".exe",
    "dll": ".dll",
}


def normalize_crypto(value: Any) -> dict[str, str | None]:
    if not isinstance(value, dict):
        return {"enc_key": None, "dec_key": None}
    return {
        "enc_key": value.get("enc_key"),
        "dec_key": value.get("dec_key"),
    }


def serialize_c2(c2_entries: Iterable[Any]) -> list[dict[str, Any]]:
    """Convert Mythic C2 objects into a stable build-time configuration."""
    result: list[dict[str, Any]] = []
    for entry in c2_entries:
        profile = entry.get_c2profile()
        params = entry.get_parameters_dict()
        clean = {
            key: normalize_crypto(value) if key.lower() == "aespsk" else value
            for key, value in params.items()
        }
        result.append(
            {
                "name": profile.get("name", ""),
                "is_p2p": bool(profile.get("is_p2p", False)),
                "parameters": clean,
            }
        )
    return result


def output_filename(filename: str, output_type: str) -> str:
    extension = OUTPUT_EXTENSIONS[output_type]
    stem = Path(filename).stem or "payload"
    return f"{stem}{extension}"


def native_config(config: dict[str, Any]) -> dict[str, Any]:
    """Flatten Mythic's selected C2 instance into the native agents' schema."""
    c2 = (config.get("c2") or [{}])[0]
    name = str(c2.get("name") or "http").lower()
    params = c2.get("parameters") or {}

    callback = str(
        params.get("callback_host")
        or params.get("callback_address")
        or params.get("host")
        or params.get("address")
        or "127.0.0.1"
    )
    parsed = urlparse(callback if "://" in callback else f"//{callback}")
    host = parsed.hostname or callback.split("/", 1)[0]
    secure = name in {"https", "httpx"} or parsed.scheme == "https"
    port = int(
        params.get("callback_port")
        or params.get("port")
        or parsed.port
        or (443 if secure else 80)
    )
    uri = str(
        params.get("callback_uri")
        or params.get("uri")
        or params.get("post_uri")
        or parsed.path
        or "/"
    )
    interval = int(
        params.get("callback_interval")
        or params.get("interval")
        or params.get("sleep")
        or 5
    )
    jitter = int(params.get("callback_jitter") or params.get("jitter") or 0)
    aes = params.get("AESPSK") or params.get("aespsk") or {}
    key = aes.get("dec_key") or aes.get("enc_key") or ""

    return {
        "payload_uuid": str(config.get("payload_uuid") or ""),
        "key_b64": str(key or ""),
        "transport": "httpx" if secure and name in {"http", "httpx", "https"} else name,
        "endpoint": f"{host}:{port}",
        "host": host,
        "port": port,
        "uri": uri if uri.startswith("/") else f"/{uri}",
        "interval_ms": max(1, interval) * 1000,
        "jitter_pct": max(0, min(100, jitter)),
        "commands": list(config.get("commands") or []),
        "output_type": str(config.get("output_type") or "exe"),
    }


@dataclass(frozen=True)
class BuildResult:
    payload: bytes
    stdout: str
    stderr: str
    filename: str


class AgentBuildError(RuntimeError):
    pass


def _artifact_candidates(root: Path, output_type: str) -> list[Path]:
    names = {
        "shellcode": ("payload.bin", "shellcode.bin", "stage.bin", "stage0.raw"),
        "raw": ("payload.bin", "raw.bin", "stage.bin", "stage0.raw"),
        "exe": ("payload.exe", "agent.exe", "stage0.exe", "stage1.exe"),
        "service_exe": (
            "payload-service.exe",
            "service.exe",
            "payload.exe",
            "stage0.exe",
            "stage1.exe",
        ),
        "dll": ("payload.dll", "agent.dll", "stage0.dll", "stage1.dll"),
    }[output_type]
    found: list[Path] = []
    for directory in (
        root / "dist",
        root / "build",
        root / "out",
        root / "target" / "x86_64-pc-windows-gnu" / "release",
        root,
    ):
        found.extend(directory / name for name in names)
    return found


def run_agent_build(
    source: Path,
    *,
    stage: str,
    output_type: str,
    filename: str,
    config: dict[str, Any],
) -> BuildResult:
    """Build an agent using the source worker's build.sh/Make/CMake contract."""
    if output_type not in OUTPUT_EXTENSIONS:
        raise AgentBuildError(f"unsupported output type: {output_type}")
    if not source.is_dir():
        raise AgentBuildError(f"agent source directory is missing: {source}")

    with tempfile.TemporaryDirectory(prefix=f"blueshell-{stage}-") as tmp:
        work = Path(tmp) / stage
        shutil.copytree(source, work)
        config_path = work / "build-config.json"
        stamped_config = native_config(config)
        config_path.write_text(
            json.dumps(stamped_config, separators=(",", ":"), sort_keys=True)
        )
        env = os.environ.copy()
        env.update(
            {
                "BLUESHELL_BUILD_CONFIG": str(config_path),
                "BLUESHELL_OUTPUT_TYPE": output_type,
                "BLUESHELL_STAGE": stage,
            }
        )

        if (work / "build.sh").is_file():
            command = ["bash", "build.sh", output_type, str(config_path)]
        elif (work / "Makefile").is_file():
            command = [
                "make",
                "release",
                f"OUTPUT_TYPE={output_type}",
                f"CONFIG={config_path}",
            ]
        elif (work / "CMakeLists.txt").is_file():
            build_dir = work / "build"
            cmake_args = [
                "cmake",
                "-S",
                str(work),
                "-B",
                str(build_dir),
                f"-DBLUESHELL_OUTPUT_TYPE={output_type}",
                f"-DBLUESHELL_CONFIG={config_path}",
                "-DCMAKE_BUILD_TYPE=Release",
            ]
            toolchain = work / "cmake" / "mingw64.cmake"
            if toolchain.is_file():
                cmake_args.extend(
                    ["-G", "Ninja", f"-DCMAKE_TOOLCHAIN_FILE={toolchain}"]
                )
            configure = subprocess.run(
                cmake_args,
                capture_output=True,
                text=True,
                env=env,
            )
            if configure.returncode:
                raise AgentBuildError(configure.stdout + configure.stderr)
            command = ["cmake", "--build", str(build_dir), "--config", "Release"]
        elif (work / "Cargo.toml").is_file():
            command = ["cargo", "build", "--release", "--target", "x86_64-pc-windows-gnu"]
        else:
            raise AgentBuildError("agent source has no build.sh, Makefile, or CMakeLists.txt")

        process = subprocess.run(
            command, cwd=work, capture_output=True, text=True, env=env
        )
        if process.returncode:
            raise AgentBuildError(process.stdout + process.stderr)
        artifact = next(
            (path for path in _artifact_candidates(work, output_type) if path.is_file()),
            None,
        )
        if artifact is None:
            raise AgentBuildError(
                f"build succeeded but produced no {output_type} artifact"
            )
        return BuildResult(
            payload=artifact.read_bytes(),
            stdout=process.stdout,
            stderr=process.stderr,
            filename=output_filename(filename, output_type),
        )


def decode_wrapped_payload(value: Any) -> bytes:
    if value is None:
        raise AgentBuildError("a wrapped payload is required")
    if isinstance(value, bytes):
        return value
    if isinstance(value, str):
        return base64.b64decode(value)
    raise AgentBuildError("wrapped payload has an unsupported representation")
