#!/usr/bin/env bash
set -euo pipefail

artifacts_dir="${1:-/artifacts}"
report_path="${artifacts_dir}/container-tests-report.md"

sections=(
  "Smoke Summary|smoke-summary.json|json"
  "Daemon Unit Tests|cargo-test-daemon.log|text"
  "Backend Unit Tests|cargo-test-backend.log|text"
  "Frontend Dependency Install|npm-ci-frontend.log|text"
  "Frontend Unit Tests|npm-test-frontend.log|text"
  "Frontend Typecheck|npm-typecheck-frontend.log|text"
  "Frontend Build|npm-build-frontend.log|text"
  "Smoke Build Log|cargo-build-smoke.log|text"
  "Smoke Daemon Log|smoke-daemon.log|text"
  "Smoke Backend Log|smoke-backend.log|text"
  "Smoke WebSocket Log|smoke-websocket.log|text"
  "Smoke WebSocket Event|smoke-websocket-event.json|json"
)

{
  printf '# Container Test Report\n\n'
  printf 'Generated: %s\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  printf 'Artifacts directory: %s\n\n' "${artifacts_dir}"

  for section in "${sections[@]}"; do
    IFS='|' read -r title file language <<<"${section}"
    path="${artifacts_dir}/${file}"

    printf '## %s\n\n' "${title}"
    if [[ -f "${path}" ]]; then
      printf '```%s\n' "${language}"
      cat "${path}"
      printf '\n```\n\n'
    else
      printf '_Missing: %s_\n\n' "${file}"
    fi
  done
} >"${report_path}"

printf 'Combined report written to %s\n' "${report_path}"
