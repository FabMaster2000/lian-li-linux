# Headless Daemon Check (Task 1.3)

## Ergebnis
Der `lianli-daemon` ist bereits headless nutzbar. Er hat keine GUI-Abhaengigkeiten und startet eigenstaendig als User-Service.
Es sind keine Code-Aenderungen notwendig.

## Start und Betrieb

### Binary
- `target/release/lianli-daemon`

### Start (manuell)
```bash
./target/release/lianli-daemon --log-level info
```
Optionale Flags:
- `--config /pfad/zur/config.json`
- `--log-level error|warn|info|debug|trace`

### Systemd (User-Service)
Service-File: `systemd/lianli-daemon.service`
```ini
[Service]
ExecStart=%h/.local/bin/lianli-daemon
```
Erwartet daher die Binary in `~/.local/bin/`.

## IPC Socket
- Pfad wird in `crates/lianli-daemon/src/ipc_server.rs` gesetzt:
  - `$XDG_RUNTIME_DIR/lianli-daemon.sock`
  - Fallback: `/tmp/lianli-daemon.sock`
- Rechte: der Socket wird auf `0666` gesetzt (lokale Clients koennen verbinden).
- Pro Verbindung: 1 Request, 1 Response (newline-delimited JSON).

## Konfiguration
- Default in `crates/lianli-daemon/src/main.rs`:
  - `$XDG_CONFIG_HOME/lianli/config.json`
  - Fallback: `~/.config/lianli/config.json` (bei fehlendem `XDG_CONFIG_HOME`)
- Beim ersten Start wird eine Default-Config erstellt, wenn keine vorhanden ist.

## GUI-Abhaengigkeiten
- Der Daemon nutzt keine GUI-Bibliotheken.
- Die Desktop-GUI (`lianli-gui`) ist ein separater Slint-Client und nicht erforderlich.

## Runtime-Voraussetzungen (headless)
- udev-Regeln fuer USB/HID (siehe `udev/99-lianli.rules`).
- Zugriff auf `/dev/hidraw*` und USB-Devices ohne root.
- Optional:
  - `ffmpeg` nur falls LCD-Video/GIF genutzt wird (Media-Pipeline).
  - `/bin/sh` falls Fan-Curves Temperatur ueber Shell-Command lesen.

## Einschraenkungen / Hinweise
- IPC-Events via `Subscribe` sind im Daemon nicht implementiert (Polling ueber `GetTelemetry`).
- OpenRGB-Server kann optional aktiviert werden (Config `rgb.openrgb_server`).

## Fazit
`lianli-daemon` ist bereits headless-faehig; Start und Betrieb sind ohne GUI moeglich.
Keine Aenderungen am Code notwendig.
