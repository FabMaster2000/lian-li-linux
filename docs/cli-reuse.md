# CLI Reuse Analysis (Task 3.2)

## Ziel

Pruefen, ob der Testclient Typen aus `lianli-shared` wiederverwenden kann und dies umsetzen.

## Ergebnis

- Wiederverwendung ist moeglich und umgesetzt.
- Der CLI nutzt direkt die IPC-Typen aus `lianli-shared`:
  - `lianli_shared::ipc::{IpcRequest, IpcResponse}`

## Warum das reicht

- IPC-Requests/Responses sind bereits in `lianli-shared` definiert und werden
vom Daemon sowie der GUI genutzt.
- Eine Duplizierung der Typen ist nicht notwendig.

## Verbleibende Mirror-Typen

- Keine: aktuell wird nichts gespiegelt oder dupliziert.

## Abhaengigkeiten

- `tools/lianli-cli/Cargo.toml` enthaelt:
  - `lianli-shared` (path dependency)
  - `serde_json`, `anyhow`, `clap`

