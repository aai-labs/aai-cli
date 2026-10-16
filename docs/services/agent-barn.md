# Agent Barn

`aai-cli agent-barn` provides 219 commands covering Agent Barn's v1 product API. It includes organization and platform administration, agents and lifecycle, configuration, templates and skills, credentials, communications, webhooks, web chat, conversations, logs, memory, restore points, costs, and resource usage.

## Configuration

```toml
[profiles.agent-barn]
provider = "agent-barn"
auth_type = "bearer_token"
base_url = "https://your-agent-barn.example/api/v1"
token_env = "AGENT_BARN_TOKEN"
# Set after inspecting auth context:
# org = "YOUR_ORGANIZATION_UUID"
```

Use the actual deployment URL, including `/api/v1` and any reverse-proxy prefix. No deployment or organization is guessed. Credentials, query strings, and fragments are forbidden in `base_url`. Store actual configurations and credentials under ignored `local/`.

Alternatively use `token_secret = "agent-barn.token"` with the existing encrypted secret store. `config profiles set/validate` supports this secret-reference form; consistent with other providers, these configuration-management commands reject environment-backed credential fields. Hand-written TOML profiles can use `token_env` as above.

```bash
aai-cli --profile agent-barn agent-barn auth context
aai-cli --profile agent-barn agent-barn --organization-id YOUR_ORGANIZATION_UUID agents list
aai-cli --profile agent-barn agent-barn discovery
aai-cli --profile agent-barn agent-barn openapi
```

`--organization-id` overrides `profile.org` for organization paths, including explicit platform organization paths. On platform endpoints that offer an organization query filter, only an explicit `--organization-id` sets that filter; `profile.org` does not silently restrict platform-wide reports. Generic requests must include the organization ID in their relative path.

Personal API keys use `Authorization: Bearer`. No login or OAuth refresh is performed, even when unrelated refresh credentials are present in a profile. READ_ONLY keys cannot mutate. FULL keys remain subject to current user membership, agent permissions, and platform administrator checks. A platform administrator still needs organization membership for tenant routes. Invalid keys return 401, denied actions 403, and concealed or missing resources 404.

## Commands and bodies

See the [complete command and endpoint reference](../../bundled/skills/aai-agent-barn/references/command-reference.md) and [bundled skill](../../bundled/skills/aai-agent-barn/SKILL.md). Path and query parameters have named flags (`agent_id` becomes `--agent-id`). Repeated array query parameters accept repeated flags. Provider enums and numeric bounds are validated locally; full JSON schema and permission validation remain server-side.

Writes accept `--json FILE` or `--json -` (stdin); the shared input reader also accepts inline JSON. Scalar body flags override corresponding JSON fields. Nested data, arrays, explicit nulls, and body fields colliding with path/query flag names use JSON. Do not use read responses as write payloads without consulting the schema. Writes are sent once, without retries. Inspect the resource after uncertain failures before retrying.

```bash
aai-cli --profile agent-barn agent-barn agents get --agent-id AGENT_ID
aai-cli --profile agent-barn agent-barn agents update --agent-id AGENT_ID --json local/agent-update.json
aai-cli --profile agent-barn agent-barn agents start --agent-id AGENT_ID
aai-cli --profile agent-barn agent-barn request get /auth/context
```

Generic `request` remains a single-response escape hatch. It accepts API-relative paths, rejects paths escaping the configured API root, and requires `--allow-write` for mutations. Redirects are not followed. Dedicated commands do not cover session login/refresh and unauthenticated password-recovery flows, key management, browser/provider callbacks, documentation pages, runtime gateways, or ingestion. Key management remains in Account settings. Public `health`, `discovery`, and `openapi` are provided as convenience commands using the configured profile.

Account `auth me change-password` is included as an authenticated account operation. It invalidates existing personal keys and returns the provider's session-token response; the CLI does not persist or switch to those tokens. Supply password fields through a local JSON file or stdin.

The deprecated connection `reconnect` route remains mapped for compatibility. Authorized active connections return 409; native transport recovery requires an agent restart.

## Collections

`--limit` caps total returned records (default 100); `--limit 0` makes no collection request. Page-number, compound conversation-cursor, and log-snapshot pagination are supported. Existing response fields remain intact; only the record array is aggregated. Bare arrays are wrapped under `results`. Provider page metadata describes the last fetched page; `_aai.pagination` describes the aggregate and continuation.

Follow `_aai.pagination.next_command` unchanged. It preserves filters and page size and uses `--page-skip` when stopping inside a page. Continuations are stateless; concurrent changes can affect results. Repeated cursors and empty pages claiming more results produce structured errors rather than loops.

Some provider collections are bounded. Organization members return at most 200 matching users, web chat threads at most 100, and initial chat history at most 500 recent messages. The CLI fetches the 200-member window regardless of its local `--limit`; narrow `--search` for larger organizations. At these caps, completion is `unknown`. Chat `--after-id` aggregates newer messages in 500-message batches; it cannot retrieve older history. Complete results mean complete within the requested provider view, not a historical audit.

## Downloads and streams

```bash
aai-cli --profile agent-barn agent-barn agents connections app-package --agent-id AGENT_ID --connection-id CONNECTION_ID --output local/app.zip
aai-cli --profile agent-barn agent-barn agents logs stream --agent-id AGENT_ID --output local/logs.sse --duration-seconds 30
aai-cli --profile agent-barn agent-barn agents web-chat stream --agent-id AGENT_ID --output local/chat.sse --duration-seconds 30
```

`--output` must name a new file in an existing directory. Files are never overwritten. Downloads preserve exact bytes; streams preserve raw SSE bytes incrementally. Stdout remains one JSON object with absolute path, content type, byte count, and termination reason. Streams default to 30 seconds; accepted durations are 1–86400 seconds, starting after response headers arrive. Read the file during capture for live inspection.

A stream reaching its duration succeeds. Unexpected disconnects and transfer errors return structured errors with `details.partial_file`; partial files are retained. No reconnection occurs. Header waits and stalled downloads time out after 30 seconds. Redirects are rejected without forwarding credentials.

On the tested cloud deployment, an idle log stream with the default `tail_lines=0` did not return response headers within the 30-second timeout. Adding `--tail-lines 1` delivered an initial log event and allowed bounded capture to complete successfully. Use this option when capturing logs from quiet agents. This observation does not establish whether the delay originates in the API or its proxy.

## Contract and maintenance

Pinned upstream revision: [`9ffaf06ddff8b80b6dcf30f3b3821cbfa150b00f`](https://github.com/aai-labs/agent-barn/tree/9ffaf06ddff8b80b6dcf30f3b3821cbfa150b00f).

The OpenAPI snapshot and provenance live in `tests/fixtures/agent_barn/`. `coverage.json` maps each upstream operation to a dedicated command or explicit exclusion; OpenAPI itself is an additional framework endpoint. The schema was generated from the upstream application without running its lifespan or connecting to live services. Some organization path parameters are absent from OpenAPI because the server resolves them dynamically; URL placeholders are authoritative.

Run `python3 scripts/generate-agent-barn-contract.py` followed by `cargo fmt` after deliberately updating the schema snapshot. Review command-name, body-field, and pagination changes before committing. Generation is offline; the CLI never constructs commands from a live schema. Endpoint-specific caps and cursor behavior must also be checked against upstream services/repositories.

Run `cargo test agent_barn`, `cargo check --all-targets --all-features`, and `scripts/run-tests.sh safe`. Offline tests exercise real local HTTP boundaries without provider credentials. Live verification remains opt-in; older deployments may return unsupported-route errors without automatic fallback.

### Live read-only smoke test

Configure a real instance in an ignored local profile, then run:

```bash
AAI_E2E_CONFIG=./local/e2e.config.toml \
AAI_E2E_AGENT_BARN_PROFILE=agent-barn \
AAI_E2E_AGENT_BARN_ORGANIZATION=YOUR_ORGANIZATION_UUID \
scripts/run-tests.sh live agent_barn_read_only_smoke
```

This checks personal-key identity, explicit organization membership, discovery, OpenAPI, agent listing, and one visible agent detail when available. It creates or changes no resources. Missing configuration fails the explicitly selected test rather than reporting a skipped check as success. This smoke test does not establish write, platform-admin, download, or streaming compatibility.
