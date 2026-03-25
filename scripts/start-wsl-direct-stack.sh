#!/usr/bin/env bash
set -euo pipefail

export HOME=/root
export XDG_CONFIG_HOME=/root/.config
export XDG_RUNTIME_DIR=/tmp/lianli-runtime

repo_dir=/mnt/c/Code/lian-li-linux
artifacts_dir="$repo_dir/artifacts"
state_dir="$artifacts_dir/runtime-state/lianli"
backend_config_path="$state_dir/backend.json"
daemon_config_path="$state_dir/config.json"
profile_store_path="$state_dir/profiles.json"
legacy_state_dir="$HOME/.config/lianli"

mkdir -p "$XDG_RUNTIME_DIR" "$XDG_CONFIG_HOME/lianli" "$artifacts_dir" "$state_dir"

if [[ ! -f "$daemon_config_path" && -f "$legacy_state_dir/config.json" ]]; then
  cp "$legacy_state_dir/config.json" "$daemon_config_path"
fi

if [[ ! -f "$profile_store_path" && -f "$legacy_state_dir/profiles.json" ]]; then
  cp "$legacy_state_dir/profiles.json" "$profile_store_path"
fi

if [[ ! -f "$backend_config_path" && -f "$legacy_state_dir/backend.json" ]]; then
  cp "$legacy_state_dir/backend.json" "$backend_config_path"
fi

cd "$repo_dir"

cargo build -p lianli-daemon -p lianli-backend

pkill -f "$repo_dir/target/debug/lianli-daemon" || true
pkill -f "$repo_dir/target/debug/lianli-backend" || true
rm -f "$XDG_RUNTIME_DIR/lianli-daemon.sock"

nohup env \
  HOME="$HOME" \
  XDG_CONFIG_HOME="$XDG_CONFIG_HOME" \
  XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
  "$repo_dir/target/debug/lianli-daemon" --config "$daemon_config_path" --log-level debug \
  >"$artifacts_dir/wsl-daemon.log" 2>&1 &
echo $! >/tmp/lianli-daemon.pid

for _ in $(seq 1 15); do
  if [[ -S "$XDG_RUNTIME_DIR/lianli-daemon.sock" ]]; then
    break
  fi
  sleep 1
done

test -S "$XDG_RUNTIME_DIR/lianli-daemon.sock"

nohup env \
  HOME="$HOME" \
  XDG_CONFIG_HOME="$XDG_CONFIG_HOME" \
  XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR" \
  LIANLI_BACKEND_CONFIG="$backend_config_path" \
  LIANLI_BACKEND_HOST=0.0.0.0 \
  LIANLI_BACKEND_PORT=9100 \
  LIANLI_DAEMON_SOCKET="$XDG_RUNTIME_DIR/lianli-daemon.sock" \
  LIANLI_DAEMON_CONFIG="$daemon_config_path" \
  LIANLI_BACKEND_PROFILE_STORE_PATH="$profile_store_path" \
  "$repo_dir/target/debug/lianli-backend" \
  >"$artifacts_dir/wsl-backend.log" 2>&1 &
echo $! >/tmp/lianli-backend.pid

for _ in $(seq 1 15); do
  if ss -ltnp | grep -q ':9100'; then
    break
  fi
  sleep 1
done

ss -ltnp | grep ':9100'
