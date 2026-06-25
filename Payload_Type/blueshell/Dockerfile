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

CMD ["python3", "main.py"]
