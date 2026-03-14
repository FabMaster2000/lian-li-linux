# Runtime Validation (Headless)

## Ziel
Schnelle Validierung, dass der `lianli-daemon` headless korrekt laeuft.

## Checkliste (manuell)
- Laeuft der Daemon?
- Existiert der Unix-Socket?
- Werden Geraete erkannt?
- Wird eine Config geladen oder erstellt?

## Schnelltest-Skript
Datei:
- `scripts/check-daemon.sh`

Ausfuehren:
```bash
bash scripts/check-daemon.sh
```

## Was das Skript prueft
- `systemctl --user is-active lianli-daemon` (optional)
- `pgrep -x lianli-daemon`
- Socket: `$XDG_RUNTIME_DIR/lianli-daemon.sock` (Fallback `/tmp`)
- Config: `$XDG_CONFIG_HOME/lianli/config.json`
- IPC Requests via Python:
  - `Ping`
  - `GetConfig`
  - `ListDevices`

## Erwartete Ausgaben
Beispiel (OK):
```
[ OK ] lianli-daemon process is running
[ OK ] IPC socket exists
[ OK ] IPC Ping
[ OK ] IPC GetConfig
[ OK ] IPC ListDevices: 3 device(s)
[ OK ] Headless runtime validation OK
```

## Typische Fehler und Hinweise
- `IPC socket not found`:
  - Daemon laeuft nicht oder `XDG_RUNTIME_DIR` ist anders.
  - Pruefe `journalctl --user -u lianli-daemon -f`.

- `permission denied` bei USB/hidraw:
  - udev-Regeln installieren/neu laden.
  - Geraet repluggen.

- `GetConfig` Fehler:
  - Erster Start? Der Daemon erstellt default config.
  - Pfad in `--config` pruefen.

## Debug-Kommandos
```bash
# Service-Status
systemctl --user status lianli-daemon

# Logs
journalctl --user -u lianli-daemon -f

# Socket
ls -la $XDG_RUNTIME_DIR/lianli-daemon.sock
```
