#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/run-tests.sh [safe|live|all] [live-test-filter]

Modes:
  safe  Run fmt, unit/integration tests, and clippy. This is the default.
  live  Run ignored live E2E tests using real provider credentials.
  all   Run safe checks, then live E2E tests.

Live mode requires:
  AAI_E2E_CONFIG=/path/to/filled-config.toml

Example:
  scripts/run-tests.sh safe
  AAI_E2E_CONFIG=./local/e2e.config.toml scripts/run-tests.sh live github_issue_crud
  AAI_E2E_CONFIG=./local/e2e.config.toml scripts/run-tests.sh all
EOF
}

mode="${1:-safe}"
filter="${2:-}"

run_safe() {
  python3 -B -m unittest discover -s scripts -p 'test_*.py'
  cargo fmt --check
  cargo test
  cargo clippy --all-targets -- -D warnings
  cargo package --list --allow-dirty | grep -q '^bundled/skills/'
}

run_live() {
  if [[ -z "${AAI_E2E_CONFIG:-}" ]]; then
    echo "AAI_E2E_CONFIG is required for live tests." >&2
    echo "Copy local/e2e.config.example.toml to local/e2e.config.toml, fill it, then export AAI_E2E_CONFIG." >&2
    exit 2
  fi

  if [[ ! -f "$AAI_E2E_CONFIG" ]]; then
    echo "AAI_E2E_CONFIG does not point to a file: $AAI_E2E_CONFIG" >&2
    exit 2
  fi

  if [[ -n "$filter" ]]; then
    cargo test --test e2e_live "$filter" -- --ignored --nocapture
  else
    cargo test --test e2e_live -- --ignored --nocapture
  fi
}

case "$mode" in
  safe)
    run_safe
    ;;
  live)
    run_live
    ;;
  all)
    run_safe
    run_live
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    echo "unknown mode: $mode" >&2
    usage >&2
    exit 2
    ;;
esac
