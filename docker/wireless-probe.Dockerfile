FROM rust:1.94-trixie AS build

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
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
COPY . .

RUN cargo build --release -p lianli-devices --example wireless_probe

FROM debian:trixie-slim AS runtime

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libusb-1.0-0 \
  && rm -rf /var/lib/apt/lists/*

COPY --from=build /work/target/release/examples/wireless_probe /usr/local/bin/wireless_probe

ENTRYPOINT ["/usr/local/bin/wireless_probe"]
