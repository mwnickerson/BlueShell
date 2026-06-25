FROM itsafeaturemythic/mythic_python_base:latest

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
       build-essential cmake curl mingw-w64 nasm ninja-build \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal \
    && /root/.cargo/bin/rustup target add x86_64-pc-windows-gnu
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /Mythic
COPY . .

# Payload builds run after the service starts and may not have outbound access.
# Follow the proven Mythic Rust-agent pattern: vendor outside /Mythic and use
# Cargo's standard root-level configuration, which survives the service mount.
RUN mkdir -p /vendor /root/.cargo \
    && cargo vendor \
      --locked \
      --versioned-dirs \
      --manifest-path /Mythic/blueshell/agent_code/stage1/Cargo.toml \
      /vendor \
      > /root/.cargo/config.toml \
    && cargo build \
      --release \
      --locked \
      --offline \
      --target x86_64-pc-windows-gnu \
      --manifest-path /Mythic/blueshell/agent_code/stage1/Cargo.toml

CMD ["python3", "main.py"]
