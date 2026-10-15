# Multica

`aai-cli multica` maps Multica's application API to dedicated commands for inbox, issues, comments, squads, agents, projects, workspaces, supporting catalogs, execution records, runtimes, and scheduling. Commands preserve provider response fields. Bare arrays are wrapped in `results`; `_aai.pagination` describes continuation and completeness.

## Configuration

```toml
[profiles.multica]
provider = "multica"
auth_type = "bearer_token"
base_url = "https://your-multica-server.example/api"
workspace = "your-workspace-slug"
token_env = "MULTICA_TOKEN"
```

Use your deployment's actual application API URL. Self-hosted examples include `http://localhost:8080/api` or `https://example.test/multica/api`. The URL must end in `/api`; credentials, query strings, and fragments are rejected. No deployment address is guessed. Supply a personal access token through `token_env`, `token_secret`, or the existing local credential configuration; never commit a real token. Multica does not use this CLI's OAuth refresh flow.

`--workspace UUID_OR_SLUG` overrides the profile. UUIDs use `X-Workspace-ID`, slugs use `X-Workspace-Slug`. Workspace administration endpoints instead identify the workspace with their required `--workspace-id UUID`. List/create workspaces do not require a selected workspace. Generic requests use the same bearer credentials and workspace headers; use `--workspace`, not workspace query parameters.

## Commands and workflows

- [Complete command/endpoint reference](../../bundled/skills/aai-multica/references/command-reference.md)
- [Inbox, commenting, assignment, squad, and administration workflows](../../bundled/skills/aai-multica/references/workflows.md)
- [Bundled agent skill](../../bundled/skills/aai-multica/SKILL.md)

Writes accept JSON files, stdin (`--json -`), or inline JSON, with typed flags overriding fields. JSON must be an object. Omitted fields and explicit nulls remain distinct. Read commands never acknowledge or archive inbox items. Writes return immediately without automatic retries; assignment, mentions, reruns, and scheduling triggers can start work.

## Pagination and progress

`--limit` is a total record cap (default 100); zero makes no collection request and does not claim completeness. Offset and cursor endpoints aggregate pages automatically. Complete lists, provider envelopes, and comment response headers are retained appropriately. For comment pagination, repeated thread roots are deduplicated and combined results are chronological.

The CLI can stop inside a provider page when a thread or inbox group returns more records than fit. In that case `_aai.pagination.next_command` includes `--page-skip` and the original server cursor; it re-fetches that page and resumes locally. Use the emitted command unchanged, including its limit and filters. This is stateless pagination, not a saved progress checkpoint. As with the upstream API, concurrent edits may change page contents between requests.

`unknown` pagination status explicitly signals a provider cap without a usable cursor or an unqueried zero-limit view. The legacy archived inbox view is bounded; use `inbox archived page` for its pageable alternative. Wakeup run history is limited to the newest ten runs. Comment/timeline/task-history truncation headers are propagated into pagination metadata. Do not infer a complete workspace audit from the recipient's inbox or from comment `since` (which filters comment creation, not edits).

Generic requests are single-page: `request get PATH` or `request post PATH --allow-write --json FILE`. Absolute paths to other origins and redirects are not followed. Billing, integration/plugin installation, and daemon lifecycle operations have no dedicated commands.

## Compatibility and verification

Contract inspected at Multica revision [`83527872634824012dd26bb183aa216fab631c05`](https://github.com/multica-ai/multica/tree/83527872634824012dd26bb183aa216fab631c05). This is the application `/api` surface, not the plugin `/v1` API. Older deployments may lack endpoints; errors are surfaced without falling back or silently changing behavior. API authorization and role restrictions remain authoritative.

Offline tests cover all dedicated routes and the inbox→issue→comment→mark-read workflow, pagination, JSON merging, scopes, and failures. For an explicitly authorized disposable live check, configure an ignored local profile and run:

```sh
AAI_E2E_CONFIG=./local/e2e.config.toml AAI_E2E_MULTICA_PROFILE=multica scripts/run-tests.sh live multica_issue_comment_lifecycle
```

The live test creates an unassigned issue, edits it, posts a plain comment, and deletes the issue. It does not acknowledge existing inbox items or launch agents intentionally.
