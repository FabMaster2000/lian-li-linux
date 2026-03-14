#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image_name="${IMAGE_NAME:-lianli-linux-test-runner}"
artifacts_dir="${ARTIFACTS_DIR:-${repo_root}/artifacts/container-tests}"

mkdir -p "${artifacts_dir}"

docker build -f "${repo_root}/docker/test-runner.Dockerfile" -t "${image_name}" "${repo_root}"

docker run --rm -t \
  -v "${repo_root}:/work" \
  -v "${artifacts_dir}:/artifacts" \
  "${image_name}" \
  bash /work/scripts/run-test-suite.sh /artifacts

printf 'Combined report: %s\n' "${artifacts_dir}/container-tests-report.md"
