# lianli-cli Usage

## Ziel

Minimaler CLI-Testclient fuer den Daemon-IPC (headless).

## Build

```bash
# im Repo-Root
cargo build -p lianli-cli --release
```

Binary:

- `target/release/lianli-cli`

## Grundlegende Nutzung

Standard-Socket:

- `$XDG_RUNTIME_DIR/lianli-daemon.sock` (Fallback: `/tmp/lianli-daemon.sock`)

Optional: eigenes Socket setzen:

```bash
./target/release/lianli-cli --socket /pfad/zum/socket.sock <command>
```

## Kommandos (MVP)

### Daemon erreichbar?

```bash
./target/release/lianli-cli ping
```

Erwartet: `pong`

### Devices listen

```bash
./target/release/lianli-cli devices
```

Erwartet: JSON-Array von `DeviceInfo`

### Device-Status (Telemetry)

```bash
./target/release/lianli-cli device-status
./target/release/lianli-cli device-status <device_id>
```

Erwartet: JSON mit `fan_rpms` und optional `coolant_temp`.

### Farbe setzen (statisch)

```bash
./target/release/lianli-cli set-color <device_id> --hex #ff0000
lianli-cli set-color <device_id> --rgb 255 0 0
```

### Effekt setzen

```bash
./target/release/lianli-cli set-effect <device_id> Rainbow --zone 0
./target/release/lianli-cli set-effect <device_id> Breathing --speed 3 --brightness 2 --hex #00ffcc
```

### Helligkeit setzen (0-100%)

```bash
./target/release/lianli-cli set-brightness <device_id> 75 --zone 0
```

Internes Mapping: 0-100% -> 0-4 (Daemon-Skala)

### Lueftergeschwindigkeit setzen (0-100%)

```bash
# alle Slots
./target/release/lianli-cli set-fan <device_id> 60
# einzelner Slot (1-4)
./target/release/lianli-cli set-fan <device_id> 60 --slot 2
```

Hinweis: Fansteuerung ist config-driven; CLI schreibt `FanConfig`.

### Config lesen/schreiben

```bash
./target/release/lianli-cli get-config
./target/release/lianli-cli save-config /pfad/zu/config.json
```

## Typische Fehler

- `connect to ...: No such file or directory`
  - Daemon laeuft nicht oder Socket-Pfad falsch.
- `daemon error: ...`
  - IPC antwortet mit Fehler (z. B. unbekanntes Device).
- `permission denied` auf USB/HID
  - udev-Regeln installieren/reloaden, Geraet neu anstecken.

## Debug-Tipps

```bash
# Daemon-Status
systemctl --user status lianli-daemon

# Logs
journalctl --user -u lianli-daemon -f

# Socket vorhanden?
ls -la $XDG_RUNTIME_DIR/lianli-daemon.sock
```

