---
name: aai-multica
description: Use aai-cli to triage Multica inbox notifications, read progress and comment threads, create and assign issues, reply to discussions, and manage squads, agents, projects, and workspace administration.
---

# Multica through aai-cli

Use `aai-cli --profile multica multica ...`. The profile needs an explicit `base_url` ending in `/api`, a personal access token, and a workspace UUID or slug. `--workspace` overrides the profile. List workspaces before selecting one when needed. Workspace administration URLs use the explicit `--workspace-id` UUID, not the profile slug.

Read [the command reference](references/command-reference.md) for the complete endpoint mapping, flags, and JSON payload guidance. Read [workflows](references/workflows.md) for inbox triage, incremental comments, assignments, squads, and agent configuration.

## Reading progress

- Start with `inbox unread-count` or `inbox unread-summary`, then `inbox list`. The list includes read and unread active notifications: inspect `read` rather than inventing an unread-only filter. Notifications belong to the authenticated recipient, not every workspace activity.
- Follow `issue_id` and available comment anchors into `issues get`, `issues timeline`, `issues comments list`, and `issues task-runs`. A board task is an **issue**; a **task** is an agent execution record.
- Comment `--since` selects newly created comments, not an audit of edits. `--recent N` selects recently active **threads**, not N individual comments. `--thread ID --tail N` reads a root plus recent replies. Use `--roots-only` or `--summary` for triage, and fetch the full thread before acting on abbreviated text.
- Inspect `_aai.pagination`. Follow `next_command` when more results exist; keep its filters and page size unchanged. `unknown` is not proof that history is complete. `inbox archived page` is the pageable archive; `inbox archived list` is a bounded legacy view. Wakeup run history is capped at the provider's latest ten runs.
- Reads do not mark notifications read. After handling an item, explicitly call `inbox read --inbox-id ID`. Use `inbox unread` to defer it. Archiving/unarchiving an issue notification affects sibling notifications for that issue; marking read/unread affects one item.

## Writing work

Discover IDs through members, squads, agents, projects, and status catalogs. Use provider status keys, not guessed display names. Assign with `issues create/update --assignee-type member|agent|squad --assignee-id ID`. Reply with `issues comments create --parent-id COMMENT_ID`.

Writes accept `--json FILE` or `--json -`; explicit flags override JSON fields. Omitted update fields remain omitted; JSON `null` is forwarded where the provider supports clearing. Nested payloads, arrays, permissions, and scheduling details use JSON. Do not reconstruct a write body by round-tripping a full read response.

Assignments, reruns, explicit mentions, and scheduling triggers may start agent work. Preserve and inspect `trigger_outcomes` and other provider receipts. Mutations return immediately and are not retried automatically. Use comment trigger preview when deciding whom a mention would invoke.

Agent environment changes use `agents env set`, not `agents update`. Preserve the provider's masking behavior; do not put environment values or credentials in reports or examples. Supply secret-bearing JSON via a local file or stdin.

For endpoints outside the dedicated commands, use `request get RELATIVE_PATH` or `request post RELATIVE_PATH --allow-write --json FILE`. Requests are single-page and scoped by the selected profile/workspace. There is no local checkpoint, background monitor, or synthetic activity feed.
