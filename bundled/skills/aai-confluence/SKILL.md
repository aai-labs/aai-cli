---
name: aai-confluence
description: Use aai-cli to inspect and manage Confluence spaces, pages, search, comments, attachments, permissions, versions, and database reads.
---

# aai-cli Confluence

Use this skill when working with Confluence Cloud through `aai-cli confluence`.

Before running commands, confirm the active profile or pass `--profile`. Prefer typed commands for spaces, pages, search, comments, attachments, permissions, versions, and database reads; use `confluence request` only for uncommon Confluence REST endpoints.

List commands aggregate pagination up to `--limit` and return trimmed provider envelopes. Get commands return full raw provider responses.

Successful output is JSON on stdout. Errors are structured JSON on stderr. See [the command reference](references/command-reference.md) for command shapes, response notes, and examples.

Database fields, views, and entries are experimental reads. Do not attempt database row edits through the generic request endpoint; Atlassian does not document these writes.
