# Repo-Analyse: lian-li-linux

## Ziel
Technische Uebersicht der Workspace-Struktur und der wichtigsten Crates/Module als Basis fuer eine spaetere Web-API (headless).

## Workspace-Uebersicht
- Root-Workspace: `Cargo.toml` mit 6 Crates in `crates/`
  - `lianli-shared`
  - `lianli-transport`
  - `lianli-devices`
  - `lianli-media`
  - `lianli-daemon`
  - `lianli-gui`

## Modul-Uebersicht (Crates)

### `lianli-shared`
- Zweck
  - Gemeinsame Typen: IPC, Konfiguration, Device-IDs, RGB/Fan-Modelle, Media- und Screen-Infos.
- Wichtigste Entry Points
  - `crates/lianli-shared/src/lib.rs` (Modul-Exports)
  - `crates/lianli-shared/src/ipc.rs` (IPC-Protokoll)
- Wichtige Datentypen
  - IPC: `IpcRequest`, `IpcResponse`, `IpcEvent`, `DeviceInfo`, `TelemetrySnapshot`
  - Config: `AppConfig`, `LcdConfig`, `FanConfig`, `FanCurve`, `RgbAppConfig`
  - Device IDs: `DeviceFamily`, `UsbId`, `KNOWN_DEVICES`
  - RGB: `RgbMode`, `RgbEffect`, `RgbDeviceConfig`, `RgbDeviceCapabilities`
- Abhaengigkeiten
  - `serde`, `serde_json`, `anyhow`

### `lianli-transport`
- Zweck
  - Low-level USB/HID Transport fuer Linux (hidapi + libusb/rusb).
- Wichtigste Entry Points
  - `crates/lianli-transport/src/lib.rs` (Exports)
- Wichtige Datentypen
  - `TransportError`
  - `HidTransport`, `RusbHidTransport`, `UsbTransport`
  - `HidBackend` (Hidapi vs Rusb)
- Abhaengigkeiten
  - `hidapi`, `rusb`, `anyhow`, `thiserror`, `tracing`

### `lianli-devices`
- Zweck
  - Geraete-Treiber/Controller fuer konkrete Lian Li Hardware (wired + wireless).
  - Device-Discovery, konkrete Implementierungen pro Familie.
- Wichtigste Entry Points
  - `crates/lianli-devices/src/lib.rs` (Modul-Exports)
  - `crates/lianli-devices/src/detect.rs` (Discovery + HID/USB-Open)
  - `crates/lianli-devices/src/traits.rs` (Fan/RGB/LCD/AIO Traits)
- Wichtige Datentypen
  - Discovery: `DetectedDevice`, `DetectedHidDevice`
  - Traits: `FanDevice`, `RgbDevice`, `LcdDevice`, `AioDevice`
  - Wireless: `WirelessController`, `DiscoveredDevice`, `WirelessFanType`
  - Device-spezifische Controller (z. B. `TlFanController`, `Ene6k77Controller`, `HydroShiftLcdController`)
- Abhaengigkeiten
  - `lianli-shared`, `lianli-transport`, `rusb`, `hidapi`, `cbc`, `des`, `hex`

### `lianli-media`
- Zweck
  - Aufbereitung von Media-Assets fuer LCD-Streaming (Images, Video/GIF, Sensor-Gauges).
- Wichtigste Entry Points
  - `crates/lianli-media/src/lib.rs` (`prepare_media_asset`)
- Wichtige Datentypen
  - `MediaAsset`, `SensorAsset`, `MediaError`
- Abhaengigkeiten
  - `lianli-shared`, `image`, `rusttype`, `tempfile`, `anyhow`

### `lianli-daemon`
- Zweck
  - Headless Daemon: Device-Discovery, Fan-Controller, RGB-Controller, LCD-Streaming, IPC-Server, OpenRGB-Server.
- Wichtigste Entry Points
  - `crates/lianli-daemon/src/main.rs` (CLI + Service-Start)
  - `crates/lianli-daemon/src/service.rs` (Haupt-Loop und Device-Management)
  - `crates/lianli-daemon/src/ipc_server.rs` (Unix-Socket IPC)
  - `crates/lianli-daemon/src/rgb_controller.rs`, `fan_controller.rs`
- Wichtige Datentypen
  - `ServiceManager` (Core-Loop)
  - `DaemonState` (IPC-Shared State)
  - `RgbController`, `FanController`
- Abhaengigkeiten
  - `lianli-shared`, `lianli-devices`, `lianli-transport`, `lianli-media`, `clap`, `serde_json`, `notify`

### `lianli-gui`
- Zweck
  - Desktop-GUI (Slint) als Referenz-Client fuer IPC.
- Wichtigste Entry Points
  - `crates/lianli-gui/src/main.rs` (UI-Callbacks)
  - `crates/lianli-gui/src/backend.rs` (IPC-Polling + Command-Dispatch)
  - `crates/lianli-gui/src/ipc_client.rs` (Socket-Client)
- Wichtige Datentypen
  - `BackendCommand`, `SharedState`
  - IPC-Nutzung via `IpcRequest` / `IpcResponse`
- Abhaengigkeiten
  - `lianli-shared`, `slint`, `serde_json`, `anyhow`

## Bestehende Funktionalitaeten (Querschnitt)

### IPC-Nachrichten
- Typen: `IpcRequest`, `IpcResponse`, `IpcEvent` in `lianli-shared/src/ipc.rs`.
- Transport: Unix Domain Socket, newline-delimited JSON.
  - Server: `lianli-daemon/src/ipc_server.rs`
  - Client: `lianli-gui/src/ipc_client.rs`
- Status: Request/Response aktiv, `Subscribe` / Events sind noch nicht implementiert (Polling ueber `GetTelemetry`).

### Device-Discovery
- Wired USB/HID: `lianli-devices/src/detect.rs` (enumerate HID/USB, stabile Device-IDs).
- Wireless: `lianli-devices/src/wireless.rs` (TX/RX Dongle, Polling + Device Records).
- Daemon nutzt Discovery in `service.rs` fuer Device-Listen und Controller-Init.

### RGB-Steuerung
- Domain-Typen: `RgbMode`, `RgbEffect`, `RgbDeviceCapabilities` in `lianli-shared`.
- Hardware: `RgbDevice` Trait + Implementierungen in `lianli-devices`.
- Daemon-Controller: `lianli-daemon/src/rgb_controller.rs` (wired + wireless, OpenRGB-Integration).
- IPC-Endpunkte: `GetRgbCapabilities`, `SetRgbEffect`, `SetRgbDirect`, `SetMbRgbSync`, `SetFanDirection`.

### Fan-Control
- Domain-Typen: `FanConfig`, `FanCurve`, `FanSpeed` in `lianli-shared`.
- Hardware: `FanDevice` Trait + Implementierungen in `lianli-devices`.
- Daemon-Loop: `lianli-daemon/src/fan_controller.rs` (Curve-Interpolation, PWM, MB-Sync).

### Persistenz / Konfiguration
- `AppConfig` in `lianli-shared/src/config.rs`.
- Speicherung: JSON-Datei (Default: `~/.config/lianli/config.json`).
- Schreiben: IPC `SetConfig`, `SetFanConfig`, `SetRgbConfig`, `SetLcdMedia`.

### Eventing / Streaming
- IPC Events: `IpcEvent` definiert, aber `Subscribe` ist im IPC-Server derzeit nicht implementiert.
- Telemetrie: `GetTelemetry` liefert Fan-RPM, Temps, OpenRGB-Status (Polling).
- LCD-Streaming: `ServiceManager` streamt Frames im Hauptloop.
- OpenRGB: eigener TCP-Server in `openrgb_server.rs` (Streaming / per-LED Updates).

## Abhaengigkeiten zwischen Modulen (vereinfacht)
- `lianli-daemon` -> `lianli-devices` -> `lianli-transport`
- `lianli-daemon` -> `lianli-media`
- `lianli-daemon` -> `lianli-shared`
- `lianli-gui` -> `lianli-shared`
- `lianli-devices` -> `lianli-shared`

## Relevante Entry-Points (Dateien)
- `crates/lianli-daemon/src/main.rs` (Daemon Start, CLI, Config-Pfad)
- `crates/lianli-daemon/src/ipc_server.rs` (IPC Socket, Request Handling)
- `crates/lianli-devices/src/detect.rs` (Device-Discovery + HID/USB Open)
- `crates/lianli-transport/src/lib.rs` (Transport-API)
- `crates/lianli-shared/src/ipc.rs` (IPC Typen)
- `crates/lianli-gui/src/ipc_client.rs` (IPC Client)

