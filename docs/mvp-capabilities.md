# MVP Capabilities (Daemon/IPC)

## Ziel
Bewertung, welche der MVP-Funktionen bereits durch den Daemon/IPC abgedeckt sind.
Grundlage: `IpcRequest`, `DeviceInfo`, `RgbController`, `FanController`, `AppConfig`.

## MVP-Funktionen und Status

| Funktion | Status | Beleg im Code | Hinweise |
| --- | --- | --- | --- |
| Geraete auflisten | vorhanden | `IpcRequest::ListDevices` -> `DeviceInfo` | Enthalt Flags zu Fan/RGB/LCD. |
| Geraetestatus lesen | teilweise vorhanden | `GetTelemetry` (RPMs, streaming, OpenRGB status) | Kein vollstaendiger RGB/LCD State; Config muss herangezogen werden. |
| Farbe setzen | vorhanden | `SetRgbEffect` (RgbEffect.mode = Static + colors) | Wired + Wireless; per-zone moeglich. |
| Effekt setzen | vorhanden | `SetRgbEffect` (RgbMode + params) | Effektliste basiert auf `RgbMode`. |
| Helligkeit setzen | vorhanden | `SetRgbEffect` (RgbEffect.brightness 0..4) | Skala 0..4 (nicht 0..100). Mapping in Web-API notwendig. |
| feste Lueftergeschwindigkeit setzen | teilweise vorhanden | Fansteuerung ist config-driven (`AppConfig.fans` + FanController) | `SetFanSpeed` existiert, wird im Daemon aktuell nicht angewandt. Erwartet Config-Update via `SetConfig`/`SetFanConfig`. |
| vorhandene Konfiguration lesen | vorhanden | `IpcRequest::GetConfig` -> `AppConfig` | JSON-Config ist umfassend. |
| Konfiguration speichern | vorhanden | `IpcRequest::SetConfig` | Daemon schreibt File + reload. |

## Zusammenfassung
- Kernfunktionen fuer RGB sind direkt im IPC vorhanden.
- Fansteuerung erfolgt aktuell ueber Config (Hintergrund-Loop), nicht per direkten IPC-Befehl.
- Geraetestatus ist partiell (Telemetry + Config); fuer Web-API muss ein zusammengesetzter State bereitgestellt werden.

## Erwartete Erweiterungen spaeter
- Direkter Fan-Set Endpunkt im Daemon (oder Backend-Mapping auf Config-Update) falls geringe Latenz gewuenscht.
- Einheitliches State-Modell (RGB-Status + LCD-Status) fuer `GET /devices/:id`.
