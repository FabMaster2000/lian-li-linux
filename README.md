<p align="center">
  <img src="assets/icons/icon.svg" width="128" height="128" alt="Lian Li Linux">
</p>

<h1 align="center">Lian Li Linux</h1>

<p align="center">
  Open-source Linux web controller for Lian Li devices.<br>
  Fan control, lighting workbenches, device inventory, profiles, LCD/media support, and daemon-backed browser access.
</p>

---

## Product Scope

The repository now ships a web-first stack:

```text
lianli-daemon          User service for hardware access, fan control, and LCD streaming
  lianli-devices       HID/USB device drivers
  lianli-transport     HID/USB backends and wireless transport
  lianli-media         Image/video/GIF encoding and sensor rendering
  lianli-shared        IPC types, config schema, device IDs

lianli-backend         HTTP/WebSocket bridge for browser clients
frontend/              React/Vite web UI
lianli-cli             CLI helper for daemon-facing smoke tests
```

The legacy native desktop GUI has been removed.
All user-facing interaction is expected to happen through the browser-delivered web application.

## Supported Devices

### Wired (HID)

| Device | Fan Control | RGB | LCD | Pump |
|--------|:-----------:|:---:|:---:|:----:|
| UNI FAN SL / AL / SL Infinity / SL V2 / AL V2 (ENE 6K77) | 4 groups | Yes | - | - |
| UNI FAN TL Controller | 4 ports | Yes | - | - |
| UNI FAN TL LCD | 4 ports | Yes | 400x400 | - |
| Galahad II Trinity AIO | Yes | Yes | - | Yes |
| HydroShift LCD AIO | Yes | Yes | 480x480 | Yes |
| Galahad II LCD / Vision AIO | Yes | Yes | 480x480 | Yes |

### Wireless (USB Bulk via TX/RX dongle)

| Device | RGB | LCD | Notes |
|--------|:---:|:---:|-------|
| UNI FAN SL V3 (LCD / LED) | Yes | 480x480 | 120mm / 140mm |
| UNI FAN TL V2 (LCD / LED) | Yes | 480x480 | 120mm / 140mm |
| UNI FAN SL-INF | Yes | - | Wireless |
| UNI FAN CL / RL120 | Yes | - | Wireless |
| HydroShift II LCD Circle | - | 480x480 | WinUSB |
| Lancool 207 Digital | - | 1472x720 | WinUSB |
| Universal Screen 8.8" | - | 1920x480 | WinUSB |

## Documentation

Start here:

- [Documentation Index](docs/README.md)
- [Technical Documentation](docs/technical/README.md)
- [Functional Documentation](docs/functional/README.md)
- [Getting Started Guide](docs/functional/user-guides/getting-started.md)
- [System Overview](docs/technical/architecture/system-overview.md)

The remaining flat files under `docs/` are transition-era technical references.
New or updated first-party documentation belongs under `docs/technical/` or `docs/functional/`.

## Build

### Prerequisites

- Rust stable
- Node.js and npm for `frontend/`
- `ffmpeg` and `ffprobe` in `PATH`
- Platform libraries for HID and USB access

```bash
# Arch
sudo pacman -S hidapi libusb ffmpeg nodejs npm

# Ubuntu / Debian
sudo apt install libhidapi-dev libusb-1.0-0-dev libudev-dev libfontconfig-dev ffmpeg nodejs npm

# Fedora
sudo dnf install hidapi-devel libusb1-devel fontconfig-devel ffmpeg nodejs npm
```

### Rust services and CLI

```bash
cargo build --release -p lianli-daemon -p lianli-backend -p lianli-cli
```

### Frontend

```bash
cd frontend
npm ci --no-audit --no-fund
npm run build
```

### Docker builder

```bash
docker build -f docker/build.Dockerfile -t lianli-linux-builder .
docker run --rm -it \
  -v "$PWD:/work" \
  -v "$PWD/target:/work/target" \
  -v "$PWD/.cache/cargo-registry:/home/builder/.cargo/registry" \
  -v "$PWD/.cache/cargo-git:/home/builder/.cargo/git" \
  lianli-linux-builder
```

## Installation

### 1. Install udev rules

```bash
sudo cp udev/99-lianli.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### 2. Install daemon and backend

```bash
mkdir -p ~/.local/bin ~/.config/systemd/user
cp target/release/lianli-daemon ~/.local/bin/
cp target/release/lianli-backend ~/.local/bin/
cp systemd/lianli-daemon.service ~/.config/systemd/user/
cp systemd/lianli-backend.service ~/.config/systemd/user/
cp systemd/lianli-stack.target ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now lianli-stack.target
```

The daemon reads `~/.config/lianli/config.json`.
The backend reads `~/.config/lianli/backend.json`.

### 3. Serve the frontend

Build the frontend and serve `frontend/dist` through a reverse proxy or static web server that also proxies:

- `/api/*` to the backend HTTP service
- `/api/ws` to the backend WebSocket endpoint

Deployment details live in [docs/deployment.md](docs/deployment.md).

## Troubleshooting

**Daemon not seeing devices**

```bash
journalctl --user -u lianli-daemon -f
sudo udevadm test /sys/bus/usb/devices/<your-device>
```

**Backend unavailable**

```bash
systemctl --user status lianli-backend
journalctl --user -u lianli-backend -f
```

**Web UI cannot connect**

- confirm the backend is reachable on the configured bind address
- confirm the reverse proxy forwards `/api/*` and `/api/ws`
- rebuild and redeploy `frontend/dist` after frontend changes

## License

MIT. See [LICENSE](LICENSE).

This project is not affiliated with Lian Li Industrial Co., Ltd.
Protocol information was obtained through reverse engineering for interoperability purposes.
