# Proxmox VM Docker Stack

This folder contains a Linux-first container setup for running `lianli-daemon` and `lianli-backend` inside a Proxmox VM.

## Recommended use

- Use this on a Debian or Ubuntu VM in Proxmox.
- Pass the Lian Li wireless dongles through to the VM first.
- Use Docker Desktop on Windows only for image and API testing, not as the main USB validation path.

For the wireless dongle pair, the relevant USB IDs are:

- TX: `0416:8040`
- RX: `0416:8041`

Example Proxmox VM passthrough:

```bash
qm set <VMID> -usb0 host=0416:8040
qm set <VMID> -usb1 host=0416:8041
```

## Files

- `compose.yml`: two-service stack for `daemon` and `backend`
- `backend.json`: backend file config inside the container
- `.env.example`: optional overrides for port, log levels, and backend auth
- `state/`: persistent daemon config and profile data created on first start

## First start

```bash
cd docker/proxmox
cp .env.example .env
docker compose up -d --build
```

The daemon writes its runtime config to:

```text
./state/lianli/config.json
```

Profiles are stored next to it as:

```text
./state/lianli/profiles.json
```

## Verification

Check service state:

```bash
cd docker/proxmox
docker compose ps
docker compose logs -f daemon
```

Check backend health:

```bash
curl -fsS http://127.0.0.1:9000/api/health
curl -fsS http://127.0.0.1:9000/api/runtime
curl -fsS http://127.0.0.1:9000/api/devices
```

## USB notes

The stack mounts `/dev/bus/usb` into the daemon container and grants USB character-device access via:

- bind mount of `/dev/bus/usb`
- device cgroup rule `c 189:* rmw`

That is enough for the wireless TX/RX dongles because they are accessed through libusb/rusb.

If you later want to run wired HID devices through `hidapi`, you may also need to pass matching `/dev/hidrawX` nodes into the daemon container.

## Auth

`backend.json` keeps auth disabled by default for the test phase.

If you want quick protection without a reverse proxy yet, edit `.env`:

```text
BACKEND_AUTH_MODE=basic
BACKEND_AUTH_USERNAME=admin
BACKEND_AUTH_PASSWORD=<strong-password>
```

Then restart:

```bash
docker compose up -d
```

## Operational notes

- This stack intentionally exposes backend port `9000` for the test phase.
- For a later LAN setup, put a reverse proxy in front and stop exposing `9000` directly.
- This folder targets a Linux VM. It is not the recommended path for Docker Desktop USB passthrough on Windows.
