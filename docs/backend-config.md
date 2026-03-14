# Backend Configuration

## Goal
Configure the web backend through a static JSON config file with optional environment overrides.

## Config File

Default path:

```text
$XDG_CONFIG_HOME/lianli/backend.json
```

Override path:

```bash
export LIANLI_BACKEND_CONFIG=/custom/path/backend.json
```

If the file does not exist, the backend starts with defaults.

## Example `backend.json`

```json
{
  "host": "0.0.0.0",
  "port": 9000,
  "socket_path": "/tmp/lianli-daemon.sock",
  "log_level": "info",
  "daemon_config_path": "/home/user/.config/lianli/config.json",
  "profile_store_path": "/home/user/.config/lianli/profiles.json",
  "auth": {
    "enabled": true,
    "mode": "basic",
    "username": "admin",
    "password": "change-me"
  }
}
```

## Supported File Keys

| Key | Default | Purpose |
|---|---|---|
| `host` | `0.0.0.0` | HTTP bind host |
| `port` | `9000` | HTTP bind port |
| `socket_path` | `$XDG_RUNTIME_DIR/lianli-daemon.sock` | Unix socket to `lianli-daemon` |
| `log_level` | `info` | Tracing filter |
| `daemon_config_path` | `$XDG_CONFIG_HOME/lianli/config.json` | Daemon config path and relative LCD/media base |
| `profile_store_path` | `<daemon-config-dir>/profiles.json` | JSON store for backend profiles |
| `auth.enabled` | `false` | Enables or disables backend auth |
| `auth.mode` | `basic` when auth is enabled without explicit mode | `basic`, `bearer`, `reverse_proxy` |
| `auth.username` | unset | Username for `basic` |
| `auth.password` | unset | Password for `basic` |
| `auth.token` | unset | Token for `bearer` |
| `auth.proxy_header` | `x-forwarded-user` | Header checked in `reverse_proxy` mode |

## Environment Overrides

These values still override file values when set:

| Variable | Purpose |
|---|---|
| `LIANLI_BACKEND_CONFIG` | Backend config file path |
| `LIANLI_BACKEND_HOST` | HTTP bind host |
| `LIANLI_BACKEND_PORT` | HTTP bind port |
| `LIANLI_DAEMON_SOCKET` | Daemon socket path |
| `LIANLI_BACKEND_LOG_LEVEL` | Log filter |
| `LIANLI_DAEMON_CONFIG` | Daemon config path |
| `LIANLI_BACKEND_PROFILE_STORE_PATH` | Profile store path |
| `LIANLI_BACKEND_AUTH_MODE` | Auth mode override |
| `LIANLI_BACKEND_AUTH_USERNAME` | Basic username override |
| `LIANLI_BACKEND_AUTH_PASSWORD` | Basic password override |
| `LIANLI_BACKEND_AUTH_TOKEN` | Bearer token override |
| `LIANLI_BACKEND_AUTH_PROXY_HEADER` | Reverse-proxy header override |

`RUST_LOG` is still accepted as fallback when `LIANLI_BACKEND_LOG_LEVEL` is unset.

## Auth Modes

### `none`
- default
- no protection on private API routes

### `basic`
- checks the HTTP `Authorization: Basic ...` header
- `auth.username` and `auth.password` are required

### `bearer`
- checks the HTTP `Authorization: Bearer ...` header
- `auth.token` is required

### `reverse_proxy`
- trusts a pre-authenticated header such as `x-forwarded-user`
- assumes the reverse proxy strips that header from direct client traffic
- `auth.proxy_header` must be a valid HTTP header name

## Restart Behavior

- Auth configuration is read once during backend startup.
- Changing `backend.json` does not reload auth live.
- To enable, disable, or change auth, edit the config and restart `lianli-backend`.

That matches the intended operating model: one restart is enough for auth changes.

## Runtime Introspection

`GET /api/runtime` reports:

- backend `config_path`
- backend `host`
- backend `port`
- backend `log_level`
- backend `profile_store_path`
- backend auth `mode`
- whether auth credentials are configured
- `reload_requires_restart: true`
- daemon `socket_path`
- daemon `config_path`

Secrets such as passwords and bearer tokens are never returned.
