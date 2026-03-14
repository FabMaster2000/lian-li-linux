FROM rust:1.94-trixie

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    jq \
    nodejs \
    npm \
    python3 \
    python3-websockets \
    build-essential \
    pkg-config \
    clang \
    cmake \
    ninja-build \
    libssl-dev \
    libhidapi-dev \
    libusb-1.0-0-dev \
    libudev-dev \
    libfontconfig-dev \
    ffmpeg \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /work

ENV CARGO_TARGET_DIR=/work/target

CMD ["bash"]
