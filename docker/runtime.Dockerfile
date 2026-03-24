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

RUN cargo build --release -p lianli-daemon -p lianli-backend

FROM debian:trixie-slim AS runtime

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    ffmpeg \
    libfontconfig1 \
    libhidapi-hidraw0 \
    libhidapi-libusb0 \
    libudev1 \
    libusb-1.0-0 \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /work

COPY --from=build /work/target/release/lianli-daemon /usr/local/bin/lianli-daemon
COPY --from=build /work/target/release/lianli-backend /usr/local/bin/lianli-backend

CMD ["sh"]
