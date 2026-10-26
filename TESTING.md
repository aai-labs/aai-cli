# Testing guidelines

All testing conventions for this Rust CLI live here.

## Canonical commands

- Full safe suite: `scripts/run-tests.sh safe`
- Release automation tests (Python 3.11+): `python3 -B -m unittest discover -s scripts -p 'test_*.py'`
- Format: `cargo fmt --check`
- Tests: `cargo test`
- Focused test: `cargo test <test-name>`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Ignored live E2E: `AAI_E2E_CONFIG=./local/e2e.config.toml scripts/run-tests.sh live [filter]`

The safe suite MUST pass before completing substantive code changes. Live tests call real provider APIs and MUST only run when explicitly requested or when the task requires live verification.

## Core principles

- Name tests for behavior so failures describe the broken contract.
- Prefer focused unit tests for parsers, request builders, auth application, pagination, merge behavior, and response shaping.
- Add integration coverage when behavior crosses CLI parsing, config resolution, HTTP request construction, or output formatting.
- For bugs, add a regression test at the deepest layer that still reproduces the failure.
- Keep tests deterministic. Do not depend on network access, real credentials, wall-clock timing, or provider state outside ignored live E2E tests.
- Preserve provider JSON response shapes in fixtures; use representative provider responses rather than simplified invented envelopes.
- Assert `_aai.pagination` status, continuation values, and next-command guidance for changed collection behavior.
- Prefer explicit setup, one primary action, and assertions on observable results.

## Unit and integration tests

Unit tests SHOULD live near the implementation in `src/` when they exercise private helpers or one module's behavior. Cross-module and CLI-level tests SHOULD live under `tests/`.

For new or changed provider commands, cover the applicable contracts:

- CLI parsing for required arguments, optional flags, and accepted values
- Typed flag and `--json <path|->` merge behavior
- Authentication headers or query parameters without exposing real secrets
- Request method, path, query parameters, and body shape
- Provider error propagation and structured error output
- Pagination aggregation and `--limit` handling for list/search commands
- Response-shape preservation after aggregation, except the documented `_aai` metadata and bare-array `results` wrapper

Use fake or local HTTP boundaries where practical. Do not mock internal functions merely to assert call counts; assert the resulting request, response, or output contract.

## Live E2E tests

For repeatable write/read/update/delete checks, use the project-local
[`live-crud-e2e`](.agents/skills/live-crud-e2e/SKILL.md) skill.

Live tests MUST be marked ignored so `cargo test` remains safe. They MUST:

- Read profiles through `AAI_E2E_CONFIG` and provider-specific profile selectors.
- Create uniquely named disposable records.
- Clean up records they create, including after partial failures when practical.
- Avoid modifying pre-existing customer data.
- Never print or commit credentials.
- Assert only stable provider behavior, not account-specific incidental data.
- For HubSpot live checks, missing scopes, unsupported auth models, or account-tier limits should assert structured `auth_error` or `unsupported_auth` JSON with remediation details, not panics or unstructured provider dumps.

Keep live tests narrow. Use them to verify authentication and representative CRUD or read flows that cannot be proven locally.

## Review checklist

- Does each changed behavior have coverage at the lowest useful layer?
- Are auth failures, validation failures, pagination boundaries, and not-found behavior covered where relevant?
- Do fixtures match real provider response shapes?
- Are live tests ignored, disposable, and credential-safe?
- Does `scripts/run-tests.sh safe` pass?
