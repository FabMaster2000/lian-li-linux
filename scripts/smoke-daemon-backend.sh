#!/usr/bin/env bash
set -euo pipefail

artifacts_dir="${1:-/artifacts}"
repo_root="${LIANLI_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
backend_port="${LIANLI_TEST_BACKEND_PORT:-19000}"
cargo_bin="${CARGO_BIN:-$(command -v cargo || true)}"

mkdir -p "${artifacts_dir}"

export LIANLI_SIM_WIRELESS="${LIANLI_SIM_WIRELESS:-slinf:3}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-${artifacts_dir}/runtime}"
export XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-${artifacts_dir}/config}"

mkdir -p "${XDG_RUNTIME_DIR}" "${XDG_CONFIG_HOME}"

cd "${repo_root}"

if [[ -z "${cargo_bin}" && -x "/root/.cargo/bin/cargo" ]]; then
  cargo_bin="/root/.cargo/bin/cargo"
fi

if [[ -z "${cargo_bin}" ]]; then
  echo "cargo not found; set CARGO_BIN or ensure cargo is on PATH" >&2
  exit 1
fi

"${cargo_bin}" build -p lianli-daemon -p lianli-backend 2>&1 | tee "${artifacts_dir}/cargo-build-smoke.log"

daemon_log="${artifacts_dir}/smoke-daemon.log"
backend_log="${artifacts_dir}/smoke-backend.log"
websocket_log="${artifacts_dir}/smoke-websocket.log"
websocket_event_payload="${artifacts_dir}/smoke-websocket-event.json"
websocket_ready_file="${artifacts_dir}/smoke-websocket.ready"
config_payload="${artifacts_dir}/config-roundtrip.json"

daemon_pid=""
backend_pid=""
websocket_pid=""

cleanup() {
  if [[ -n "${websocket_pid}" ]] && kill -0 "${websocket_pid}" 2>/dev/null; then
    kill "${websocket_pid}" || true
    wait "${websocket_pid}" 2>/dev/null || true
  fi

  if [[ -n "${backend_pid}" ]] && kill -0 "${backend_pid}" 2>/dev/null; then
    kill "${backend_pid}" || true
    wait "${backend_pid}" 2>/dev/null || true
  fi

  if [[ -n "${daemon_pid}" ]] && kill -0 "${daemon_pid}" 2>/dev/null; then
    kill "${daemon_pid}" || true
    wait "${daemon_pid}" 2>/dev/null || true
  fi
}

trap cleanup EXIT

./target/debug/lianli-daemon --log-level info >"${daemon_log}" 2>&1 &
daemon_pid=$!

for _ in $(seq 1 50); do
  if [[ -S "${XDG_RUNTIME_DIR}/lianli-daemon.sock" ]]; then
    break
  fi
  sleep 0.2
done

if [[ ! -S "${XDG_RUNTIME_DIR}/lianli-daemon.sock" ]]; then
  echo "daemon socket was not created at ${XDG_RUNTIME_DIR}/lianli-daemon.sock" >&2
  exit 1
fi

LIANLI_BACKEND_HOST=127.0.0.1 \
LIANLI_BACKEND_PORT="${backend_port}" \
LIANLI_DAEMON_SOCKET="${XDG_RUNTIME_DIR}/lianli-daemon.sock" \
LIANLI_DAEMON_CONFIG="${XDG_CONFIG_HOME}/lianli/config.json" \
./target/debug/lianli-backend >"${backend_log}" 2>&1 &
backend_pid=$!

base_url="http://127.0.0.1:${backend_port}"

for _ in $(seq 1 50); do
  if curl -fsS "${base_url}/api/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done

health="$(curl -fsS "${base_url}/api/health")"
daemon_status="$(curl -fsS "${base_url}/api/daemon/status")"
devices="$(curl -fsS "${base_url}/api/devices")"
device_id="$(printf '%s' "${devices}" | jq -r '.[0].id')"

if [[ -z "${device_id}" || "${device_id}" == "null" ]]; then
  echo "no simulated device was returned by /api/devices" >&2
  exit 1
fi

lighting_before="$(curl -fsS "${base_url}/api/devices/${device_id}/lighting")"
rm -f "${websocket_event_payload}" "${websocket_ready_file}"
python3 "${repo_root}/scripts/await-websocket-event.py" \
  "ws://127.0.0.1:${backend_port}/api/ws" \
  "lighting.changed" \
  "${websocket_event_payload}" \
  "${websocket_ready_file}" \
  10 >"${websocket_log}" 2>&1 &
websocket_pid=$!

for _ in $(seq 1 50); do
  if [[ -f "${websocket_ready_file}" ]]; then
    break
  fi
  if ! kill -0 "${websocket_pid}" 2>/dev/null; then
    wait "${websocket_pid}"
  fi
  sleep 0.2
done

if [[ ! -f "${websocket_ready_file}" ]]; then
  echo "websocket smoke helper did not become ready" >&2
  exit 1
fi

lighting_set="$(curl -fsS -X POST "${base_url}/api/devices/${device_id}/lighting/color" \
  -H 'content-type: application/json' \
  -d '{"color":{"hex":"#112233"}}')"
lighting_after="$(curl -fsS "${base_url}/api/devices/${device_id}/lighting")"
wait "${websocket_pid}"
websocket_pid=""
websocket_event="$(cat "${websocket_event_payload}")"
jq -e \
  --arg device_id "${device_id}" \
  '.type == "lighting.changed"
   and .source == "api"
   and .device_id == $device_id
   and .data.reason == "color_set"
   and .data.color == "#112233"' \
  "${websocket_event_payload}" >/dev/null

fans_set="$(curl -fsS -X POST "${base_url}/api/devices/${device_id}/fans/manual" \
  -H 'content-type: application/json' \
  -d '{"percent":42}')"
fans_after="$(curl -fsS "${base_url}/api/devices/${device_id}/fans")"

curl -fsS "${base_url}/api/config" >"${config_payload}"
config_get="$(cat "${config_payload}")"
config_post="$(curl -fsS -X POST "${base_url}/api/config" \
  -H 'content-type: application/json' \
  --data-binary @"${config_payload}")"

jq -n \
  --arg device_id "${device_id}" \
  --argjson health "${health}" \
  --argjson daemon_status "${daemon_status}" \
  --argjson devices "${devices}" \
  --argjson lighting_before "${lighting_before}" \
  --argjson lighting_set "${lighting_set}" \
  --argjson lighting_after "${lighting_after}" \
  --argjson websocket_event "${websocket_event}" \
  --argjson fans_set "${fans_set}" \
  --argjson fans_after "${fans_after}" \
  --argjson config_get "${config_get}" \
  --argjson config_post "${config_post}" \
  '{
    device_id: $device_id,
    health: $health,
    daemon_status: $daemon_status,
    devices: $devices,
    lighting_before: $lighting_before,
    lighting_set: $lighting_set,
    lighting_after: $lighting_after,
    websocket_event: $websocket_event,
    fans_set: $fans_set,
    fans_after: $fans_after,
    config_get: $config_get,
    config_post: $config_post
  }' >"${artifacts_dir}/smoke-summary.json"

printf 'Smoke test finished successfully for device %s\n' "${device_id}"
