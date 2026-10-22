---
name: aai-agent-barn
description: Use aai-cli to manage Agent Barn agents, configuration, templates, skills, organization access, activity, communications, costs, and platform administration through its product API.
---

# Agent Barn through aai-cli

Profiles may use the managed credential gateway for typed Agent Barn commands. The generic `request` command and streaming captures are unavailable in gateway mode; see the repository gateway documentation for setup.

Use `aai-cli --profile agent-barn agent-barn ...`. Configure an explicit API `base_url` ending in `/api/v1`, `auth_type = "bearer_token"`, and an existing personal API key through `token_env` or `token_secret`. Changing the account password invalidates existing keys; the CLI never persists the session tokens returned by that account operation. Supply password changes through JSON on stdin or a local file. Key creation and revocation remain in Agent Barn Account settings; no CLI login or refresh flow is needed.

Start with `auth context` to discover the caller, memberships, key access mode, and platform authority. Set `--organization-id` or `profile.org` explicitly before accessing organization resources. Platform access does not imply organization membership. READ_ONLY keys cannot mutate; FULL keys still require the user's organization and agent permissions. Missing or inaccessible resources can return 404; denied actions return 403.

Read [the command reference](references/command-reference.md) for exact resource commands and endpoint mappings. Use `discovery` or `openapi` to inspect the deployed API; dedicated commands follow the pinned v1 contract.

## Agent workflows

- Inspect `agents list`, `agents get`, `agents healthz get`, and `agents diagnostics get` before lifecycle changes. `agents start`, `agents stop`, and `agents managed-update` change running workloads.
- Configuration drafts are distinct from active configuration. Create/update a draft, review it, then publish or select a version. Editing shared templates or skills does not automatically update a running agent's pins.
- Use `agents share get/set` and organization membership commands for access. Inspect returned allowed actions rather than assuming a FULL key grants all permissions.
- Read conversations, activity, tool calls, and cost summaries to understand execution. Web chat message creation sends work to an agent; replaying it can duplicate work.
- The deprecated connection `reconnect` endpoint returns 409 for authorized active connections. Native transport recovery requires restarting the agent; do not repeatedly call reconnect.

## Requests and pagination

Writes accept `--json FILE` or `--json -`. Scalar body flags override fields in the JSON object; use JSON for nested objects, arrays, and explicit nulls. Keep credential-bearing payloads in ignored local files or stdin. Preserve provider masking and do not copy secret values into reports.

Collections aggregate up to `--limit` (default 100). Follow `_aai.pagination.next_command` with the same filters and page size. `--page-skip` resumes inside a provider page. `unknown` is not proof of complete history: web chat initially returns at most 500 recent messages and 100 threads. Message `--after-id` reads newer messages, not older history.

`request get RELATIVE_PATH` is a single-response escape hatch. Generic mutations require `--allow-write`. Paths are relative to the configured API root and must contain their organization ID explicitly. Writes are not automatically retried; after an uncertain network result, inspect the resource before retrying.

## Downloads and streams

App packages and log/chat streams require `--output PATH` to a new file. Streams preserve raw SSE bytes and stop after `--duration-seconds` (default 30). Stdout contains JSON file metadata, never raw binary or streaming events. Read a stream file while capture runs if live inspection is needed. Unexpected disconnects return structured errors with the partial-file path; no automatic reconnect occurs.
