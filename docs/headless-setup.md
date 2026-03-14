# Headless Setup (Ubuntu Server)

## Ziel
Bau und Betrieb des `lianli-daemon` auf einer Ubuntu-Server-VM ohne GUI.
Die Desktop-GUI wird nicht gebaut oder gestartet.

## Systemabhaengigkeiten (Ubuntu)

### Build-Tools
- `build-essential` (gcc/g++ fuer C++ Build-Dependencies)
- `pkg-config` (fuer Library-Erkennung bei Cargo)
- `git` (Submodules)

### Libraries
- `libhidapi-dev` (hidapi fuer HID-Devices)
- `libusb-1.0-0-dev` (libusb/rusb fuer USB Bulk + HID)
- `libudev-dev` (udev API)
- `libfontconfig-dev` (Fonts fuer Sensor-Gauges, falls TTF genutzt)

### Runtime Tools
- `ffmpeg` (nur noetig, wenn Video-Dateien fuer LCD genutzt werden)

Beispiel-Installation:
```bash
sudo apt update
sudo apt install -y build-essential pkg-config git \
  libhidapi-dev libusb-1.0-0-dev libudev-dev libfontconfig-dev \
  ffmpeg
```

## Rust Toolchain
- Erforderlich: Rust stable >= 1.75 (siehe README)
- Empfehlung: rustup verwenden

Pruefen:
```bash
rustc --version
```

## Repository vorbereiten
Die C++-Vendor-Libs werden als Git-Submodule bezogen.
```bash
git clone --recurse-submodules https://github.com/sgtaziz/lian-li-linux.git
cd lian-li-linux
# Falls ohne Submodule geklont:
git submodule update --init --recursive
```

## Build (headless, nur Daemon)
Nur den Daemon bauen, um GUI-Dependencies zu vermeiden:
```bash
cargo build -p lianli-daemon --release
```
Binary:
- `target/release/lianli-daemon`

Hinweis: Beim Build wird C++ kompiliert (`vendor/tuz_wrapper.cpp` u.a.).

## udev-Regeln installieren (noetig fuer USB/HID ohne root)
```bash
sudo cp udev/99-lianli.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```
Danach Geraete einmal ab- und wieder anstecken.

## Start (manuell)
```bash
./target/release/lianli-daemon --log-level info
```
Optionale Flags:
- `--config /pfad/zur/config.json`
- `--log-level error|warn|info|debug|trace`

## Start (systemd --user)
```bash
mkdir -p ~/.local/bin
cp target/release/lianli-daemon ~/.local/bin/

mkdir -p ~/.config/systemd/user
cp systemd/lianli-daemon.service ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable --now lianli-daemon
```
Optional (wenn Dienst nach Logout laufen soll):
```bash
loginctl enable-linger $USER
```

Logs:
```bash
journalctl --user -u lianli-daemon -f
```

## Wichtige Pfade
- Config (Default):
  - `$XDG_CONFIG_HOME/lianli/config.json`
  - Falls `XDG_CONFIG_HOME` nicht gesetzt: `~/.config/lianli/config.json`
- IPC Socket (Daemon):
  - `$XDG_RUNTIME_DIR/lianli-daemon.sock`
  - Falls `XDG_RUNTIME_DIR` nicht gesetzt: `/tmp/lianli-daemon.sock`

## Moegliche Rechteprobleme
- **Symptom:** `permission denied` auf `/dev/hidraw*` oder USB-Devices
  - udev-Regeln installiert?
  - `udevadm control --reload-rules` + `udevadm trigger` ausgefuehrt?
  - Geraet repluggen

- **Symptom:** keine HID-Devices gefunden
  - udev-Regeln enthalten `new_id` Eintraege fuer usbhid (siehe `udev/99-lianli.rules`)
  - Replug nach Regel-Reload

## Runtime-Checks (schnell)
```bash
# Daemon-Prozess?
ps aux | grep lianli-daemon

# Socket vorhanden?
ls -la $XDG_RUNTIME_DIR/lianli-daemon.sock
# oder
ls -la /tmp/lianli-daemon.sock
```

## Hinweise fuer headless Betrieb
- Die GUI wird nicht gebaut oder benoetigt.
- LCD-Video/GIF erfordert `ffmpeg` im PATH.
- Sensor-Gauges nutzen TTF nur, wenn `font_path` in der Config gesetzt ist.
