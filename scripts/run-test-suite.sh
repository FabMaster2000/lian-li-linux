#!/usr/bin/env bash
set -euo pipefail

artifacts_dir="${1:-/artifacts}"
repo_root="${LIANLI_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cargo_bin="${CARGO_BIN:-$(command -v cargo || true)}"
npm_bin="${NPM_BIN:-$(command -v npm || true)}"
frontend_workdir="${FRONTEND_WORKDIR:-${artifacts_dir}/frontend-work}"

mkdir -p "${artifacts_dir}"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${repo_root}/target}"
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

if [[ -z "${npm_bin}" ]]; then
  echo "npm not found; set NPM_BIN or ensure npm is on PATH" >&2
  exit 1
fi

"${cargo_bin}" test -p lianli-daemon -- --nocapture 2>&1 | tee "${artifacts_dir}/cargo-test-daemon.log"
"${cargo_bin}" test -p lianli-backend -- --nocapture 2>&1 | tee "${artifacts_dir}/cargo-test-backend.log"

rm -rf "${frontend_workdir}"
mkdir -p "${frontend_workdir}"
cp -a "${repo_root}/frontend/." "${frontend_workdir}/"

pushd "${frontend_workdir}" >/dev/null
"${npm_bin}" ci --no-audit --no-fund 2>&1 | tee "${artifacts_dir}/npm-ci-frontend.log"
"${npm_bin}" run test:run 2>&1 | tee "${artifacts_dir}/npm-test-frontend.log"
"${npm_bin}" run typecheck 2>&1 | tee "${artifacts_dir}/npm-typecheck-frontend.log"
"${npm_bin}" run build -- --outDir "${artifacts_dir}/frontend-dist" 2>&1 | tee "${artifacts_dir}/npm-build-frontend.log"
popd >/dev/null

bash "${repo_root}/scripts/smoke-daemon-backend.sh" "${artifacts_dir}"
bash "${repo_root}/scripts/generate-container-report.sh" "${artifacts_dir}"

printf 'Container test suite finished. Artifacts: %s\n' "${artifacts_dir}"
