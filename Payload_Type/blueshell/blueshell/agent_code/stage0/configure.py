#!/usr/bin/env python3
import json
import sys
import uuid
from pathlib import Path

path = Path(sys.argv[1])
cfg = json.loads(path.read_text()) if path.is_file() else {
    "payload_uuid": "00000000-0000-0000-0000-000000000000",
    "key_b64": "",
    "transport": "http",
    "host": "127.0.0.1",
    "port": 80,
    "uri": "/",
    "interval_ms": 5000,
    "jitter_pct": 0,
}
payload_id = ", ".join(f"0x{x:02x}" for x in uuid.UUID(cfg["payload_uuid"]).bytes)
key = cfg.get("key_b64", "")
import base64
key_bytes = base64.b64decode(key) if key else b""
key_bytes = (key_bytes + bytes(32))[:32]
key_data = ", ".join(f"0x{x:02x}" for x in key_bytes)
transport = {"http": 1, "https": 2, "httpx": 2, "tcp": 3, "smb": 4}.get(cfg["transport"], 1)
host = cfg["host"].replace("\\", "\\\\").replace('"', '\\"')
uri = cfg["uri"].replace("\\", "\\\\").replace('"', '\\"')
text = f"""#define STAGE0_STAMP_TRANSPORT {transport}
#define STAGE0_STAMP_PORT {int(cfg["port"])}
#define STAGE0_STAMP_SLEEP {min(65535, int(cfg["interval_ms"]))}
#define STAGE0_STAMP_JITTER {int(cfg["jitter_pct"])}
#define STAGE0_STAMP_SECURE {1 if transport == 2 else 0}
#define STAGE0_STAMP_HOST L"{host}"
#define STAGE0_STAMP_PATH L"{uri}"
#define STAGE0_STAMP_PAYLOAD_ID {{{payload_id}}}
#define STAGE0_STAMP_KEY {{{key_data}}}
"""
Path(sys.argv[2]).write_text(text)
