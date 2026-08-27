FROM debian:13-slim

ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

RUN apt-get update && apt-get install -y \
    curl git sudo build-essential pkg-config libusb-1.0-0-dev libudev-dev clang lld \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 segotep && \
    useradd -m -u 1000 -g segotep -s /bin/bash segotep && \
    echo "segotep ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/segotep && \
    usermod -aG plugdev,sudo segotep 2>/dev/null || true

USER segotep
