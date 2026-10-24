FROM debian:13-slim

ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl git sudo build-essential pkg-config libusb-1.0-0-dev libudev-dev clang lld ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 dev && \
    useradd -m -u 1000 -g dev -s /bin/bash dev && \
    echo "dev ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/dev && \
    usermod -aG plugdev,sudo dev 2>/dev/null || true

RUN mkdir -p /usr/local/cargo && \
    chown -R dev:dev /usr/local/cargo

SHELL ["/bin/bash", "-c"]
USER dev

ENV CARGO_HOME=/usr/local/cargo
ENV PATH="/usr/local/cargo/bin:${PATH}"

# Rust stable toolchain + analyzer/fmt/clippy baked into the image,
# matching other fleet Rust projects (gap, freshdesk).
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path && \
    /usr/local/cargo/bin/rustup component add rust-analyzer rustfmt clippy && \
    /usr/local/cargo/bin/rustup target add x86_64-unknown-linux-musl
