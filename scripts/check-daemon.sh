#!/usr/bin/env bash

set -u

fail=0

info() { printf "[INFO] %s\n" "$*"; }
ok() { printf "[ OK ] %s\n" "$*"; }
warn() { printf "[WARN] %s\n" "$*"; }
failf() { printf "[FAIL] %s\n" "$*"; fail=1; }

runtime_dir="${XDG_RUNTIME_DIR:-/tmp}"
socket_path="${runtime_dir}/lianli-daemon.sock"
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
config_path="${config_home}/lianli/config.json"

info "Runtime dir: ${runtime_dir}"
info "Socket path: ${socket_path}"
info "Config path: ${config_path}"

# Check systemd user service (optional)
if command -v systemctl >/dev/null 2>&1; then
  if systemctl --user is-active --quiet lianli-daemon; then
    ok "systemd --user service is active"
  else
    warn "systemd --user service not active (daemon may be started manually)"
  fi
else
  warn "systemctl not found; skipping service check"
fi

# Check daemon process
if pgrep -x lianli-daemon >/dev/null 2>&1; then
  ok "lianli-daemon process is running"
else
  failf "lianli-daemon process not running"
fi

# Check IPC socket
if [ -S "$socket_path" ]; then
  ok "IPC socket exists"
else
  failf "IPC socket not found"
fi

# Check config file
if [ -f "$config_path" ]; then
  ok "config file exists"
else
  warn "config file not found (daemon should create on first start)"
fi

# IPC validation via python (Ping, GetConfig, ListDevices)
if command -v python3 >/dev/null 2>&1; then
  export LL_SOCKET="$socket_path"
  python3 - <<'PY'
import json
import os
import socket
import sys

path = os.environ.get("LL_SOCKET")
if not path:
    print("[FAIL] LL_SOCKET not set")
    sys.exit(1)

def send(req):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(2)
    s.connect(path)
    payload = json.dumps(req) + "\n"
    s.sendall(payload.encode("utf-8"))
    s.shutdown(socket.SHUT_WR)
    data = b""
    while True:
        try:
            chunk = s.recv(4096)
        except socket.timeout:
            break
        if not chunk:
            break
        data += chunk
    s.close()
    line = data.splitlines()[0] if data else b""
    if not line:
        raise RuntimeError("no response")
    return json.loads(line.decode("utf-8"))

def check_ok(resp):
    return resp.get("status") == "ok"

try:
    resp = send({"method": "Ping"})
    if check_ok(resp):
        print("[ OK ] IPC Ping")
    else:
        print("[FAIL] IPC Ping error: %s" % resp)
        sys.exit(2)

    resp = send({"method": "GetConfig"})
    if check_ok(resp) and resp.get("data") is not None:
        print("[ OK ] IPC GetConfig")
    else:
        print("[FAIL] IPC GetConfig error: %s" % resp)
        sys.exit(3)

    resp = send({"method": "ListDevices"})
    if check_ok(resp):
        data = resp.get("data") or []
        print("[ OK ] IPC ListDevices: %d device(s)" % len(data))
    else:
        print("[FAIL] IPC ListDevices error: %s" % resp)
        sys.exit(4)

except Exception as e:
    print("[FAIL] IPC check failed: %s" % e)
    sys.exit(10)

PY
  if [ $? -eq 0 ]; then
    ok "IPC request checks passed"
  else
    failf "IPC request checks failed"
  fi
else
  warn "python3 not found; skipping IPC request checks"
fi

if [ $fail -eq 0 ]; then
  ok "Headless runtime validation OK"
  exit 0
else
  failf "Headless runtime validation FAILED"
  exit 1
fi
