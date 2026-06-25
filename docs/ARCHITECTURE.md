# BlueShell architecture

## Payload family

BlueShell exposes two independent Mythic payload types while sharing the same
wire contract and server-side command names.

### Stage 0

Stage 0 is the compact C/C++ payload. It is expected to:

- check in and retrieve tasking without stage 1;
- execute the minimal operator command set;
- move files in either direction;
- execute COFF objects in process;
- carry SOCKS and reverse-port-forward frames;
- retrieve and start a stage 1 payload.

Its release build avoids a runtime dependency on Rust or the C++ standard
library and keeps configuration in a replaceable build-time blob.

### Stage 1

Stage 1 is the modular Rust payload. A small command core is always present;
additional command groups are selected using Cargo features from the Mythic
build request. Transport and command dispatch are traits so later Linux support
does not require replacing the Mythic protocol implementation.

## Wire contract

Both stages use Mythic's normal message format:

```text
base64(payload-or-callback UUID || encrypted JSON)
```

When encryption keys are configured, the encrypted body is:

```text
AES-256-CBC IV || ciphertext || HMAC-SHA256
```

The top-level request/response models preserve `delegates`, `socks`, and
`rpfwd` arrays alongside task responses. Proxy data is therefore serviced on
every check-in cycle rather than only while a proxy command is active.

## Transport boundary

- `http` and `httpx` are egress transports.
- `smb` and `tcp` implement Mythic peer-to-peer delegate forwarding.
- Transport configuration is generated from the selected C2 profile and
  embedded into the build's private configuration blob.
- A payload build supports one selected C2 instance. The shared abstractions do
  not imply that multiple live transports are compiled into every artifact.

## Output pipeline

The source payload is compiled as either a native executable or position-
independent code. DLL and service outputs add purpose-built entry shims around
the same core. Raw output is extracted from the PIC build after relocation and
import checks succeed.

Every output is generated from a temporary build copy. Source files in the
installed service are never stamped in place.

## OPSEC invariants

- Debug output is compile-time gated and disabled by default.
- Release builds strip symbols and do not contain the repository, framework,
  or author name in target-side strings.
- URI paths, headers, sleep, jitter, pipe names, and TCP ports come from the
  build configuration.
- Process creation, file writes, proxy creation, COFF execution, token
  operations, and injection-oriented modules use Mythic OPSEC pre-checks.
- Build seeds may alter non-semantic layout and encrypted string material, but
  never alter the protocol contract.

