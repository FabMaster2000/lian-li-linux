# Deployment

## Goal
Run `lianli-daemon`, `lianli-backend`, and the web UI in a stable LAN setup with clear startup order, restart behavior, and diagnostics.

## Recommended Topology

- `lianli-daemon` runs as a user `systemd` service and owns hardware access.
- `lianli-backend` runs as a user `systemd` service and talks to the daemon over the Unix socket.
- The frontend is built once with `vite build` and served as static files by a reverse proxy such as Nginx.
- The reverse proxy terminates HTTP, serves `frontend/dist`, proxies `/api/*` and `/api/ws`, and optionally performs auth in front of the backend.

## Startup Order

1. Install udev rules and reload them.
2. Start `lianli-daemon`.
3. Confirm the daemon socket exists and the daemon can see devices.
4. Start `lianli-backend`.
5. Build the frontend with `cd frontend && npm run build`.
6. Start or reload the reverse proxy that serves `frontend/dist` and proxies the backend.

## Pre-Flight

- Build binaries:

```bash
cargo build --release -p lianli-daemon -p lianli-backend
cd frontend && npm run build
```

- Install binaries:

```bash
install -Dm755 target/release/lianli-daemon ~/.local/bin/lianli-daemon
install -Dm755 target/release/lianli-backend ~/.local/bin/lianli-backend
```

- Install udev rules:

```bash
sudo cp udev/99-lianli.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## Runtime Paths

- Daemon config: `~/.config/lianli/config.json`
- Backend config: `~/.config/lianli/backend.json`
- Default backend profile store: `~/.config/lianli/profiles.json`
- Default daemon socket: `$XDG_RUNTIME_DIR/lianli-daemon.sock`

## Backend Config

Example `~/.config/lianli/backend.json`:

```json
{
  "host": "127.0.0.1",
  "port": 9000,
  "socket_path": "/run/user/1000/lianli-daemon.sock",
  "log_level": "info",
  "daemon_config_path": "/home/user/.config/lianli/config.json",
  "profile_store_path": "/home/user/.config/lianli/profiles.json",
  "auth": {
    "enabled": true,
    "mode": "reverse_proxy",
    "proxy_header": "x-forwarded-user"
  }
}
```

Notes:

- For LAN use behind a reverse proxy, `host: 127.0.0.1` is the safer default.
- Auth is optional.
- Auth is read only at backend startup.
- Changing `backend.json` requires a backend restart to activate or deactivate auth.

Full config reference: `docs/backend-config.md`

## Service Definitions

### Daemon

Already in the repo: `systemd/lianli-daemon.service`

Install it as a user service:

```bash
mkdir -p ~/.config/systemd/user
cp systemd/lianli-daemon.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now lianli-daemon
```

### Backend

Provided in the repo: `systemd/lianli-backend.service`

```ini
[Unit]
Description=Lian Li Web Backend
After=lianli-daemon.service
Requires=lianli-daemon.service

[Service]
Type=simple
ExecStart=%h/.local/bin/lianli-backend
Restart=on-failure
RestartSec=5
Environment=LIANLI_BACKEND_CONFIG=%h/.config/lianli/backend.json

[Install]
WantedBy=default.target
```

Install it as:

```bash
mkdir -p ~/.config/systemd/user
cp systemd/lianli-backend.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now lianli-backend
```

### Combined Start

Provided in the repo: `systemd/lianli-stack.target`

Use it when you want a single user target that starts both daemon and backend:

```bash
mkdir -p ~/.config/systemd/user
cp systemd/lianli-daemon.service ~/.config/systemd/user/
cp systemd/lianli-backend.service ~/.config/systemd/user/
cp systemd/lianli-stack.target ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now lianli-stack.target
```

Notes:

- `lianli-stack.target` starts `lianli-daemon.service` and `lianli-backend.service`
- you can still manage both services individually
- there is intentionally no dedicated frontend service in the repo

### Frontend

- Recommended: no dedicated frontend process
- Serve `frontend/dist` statically from the reverse proxy
- Rebuild and reload the proxy when frontend assets change

## Reverse Proxy Example

Example Nginx site:

```nginx
server {
    listen 80;
    server_name lianli.lan;

    root /opt/lianli/frontend/dist;
    index index.html;

    location / {
        try_files $uri /index.html;
    }

    location /api/ws {
        proxy_pass http://127.0.0.1:9000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-User $remote_user;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:9000;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-User $remote_user;
    }
}
```

If backend auth mode is `reverse_proxy`:

- the proxy must strip any incoming `X-Forwarded-User` from clients
- the proxy must set that header itself after successful auth
- the backend should stay bound to `127.0.0.1`

If backend auth mode is `basic` or `bearer`:

- the proxy can pass the `Authorization` header through
- backend auth remains the enforcing layer

## Firewall

Recommended LAN posture:

- expose only the reverse proxy port to the network
- do not expose backend port `9000` directly when a proxy is used
- keep the backend bound to `127.0.0.1` unless direct access is required

Example with `ufw`:

```bash
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw deny 9000/tcp
```

If you intentionally expose the backend directly:

- open `9000/tcp`
- enable backend auth
- accept that static frontend hosting must be solved separately

## Validation

Check daemon:

```bash
systemctl --user status lianli-daemon
ls -la "$XDG_RUNTIME_DIR/lianli-daemon.sock"
```

Check backend:

```bash
systemctl --user status lianli-backend
curl -fsS http://127.0.0.1:9000/api/health
curl -fsS http://127.0.0.1:9000/api/runtime
```

Check reverse proxy:

```bash
curl -I http://lianli.lan/
curl -I http://lianli.lan/api/health
```

## Logs

Daemon logs:

```bash
journalctl --user -u lianli-daemon -f
```

Backend logs:

```bash
journalctl --user -u lianli-backend -f
```

Reverse proxy logs:

```bash
sudo journalctl -u nginx -f
```

## Error Diagnosis

### Backend reports daemon offline

- verify `lianli-daemon` is running
- verify `backend.json` `socket_path` matches the daemon socket
- verify the socket exists under `$XDG_RUNTIME_DIR`

### API returns `401 UNAUTHORIZED`

- check `backend.json` auth mode
- if using backend auth, confirm the client sends the correct `Authorization` header
- if using `reverse_proxy`, confirm the proxy injects the configured header
- restart `lianli-backend` after auth changes

### Frontend loads but no data appears

- confirm `/api/health` and `/api/runtime` are reachable through the proxy
- confirm the proxy forwards `/api/ws`
- confirm the frontend was rebuilt after recent changes

### WebSocket live updates do not work

- verify the proxy has `Upgrade` and `Connection` handling for `/api/ws`
- verify auth also works for the WebSocket route
- check backend logs for websocket disconnects

### No devices found

- re-check udev rules
- reconnect the device
- confirm the daemon sees hardware before debugging the backend or frontend

## Operational Notes

- The backend does not live-reload config.
- Auth changes need only a backend restart.
- Daemon socket or storage path changes also require a backend restart.
- Frontend asset changes require a new `frontend/dist` build and proxy reload only if file ownership or paths changed.
