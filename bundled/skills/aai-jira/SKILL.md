---
name: aai-jira
description: Use aai-cli to search and manage Jira issues, Jira Product Discovery ideas, projects, boards, sprints, comments, and attachments.
---

# aai-cli Jira

Profiles may use the managed credential gateway. Generic requests are unavailable in gateway mode; see the repository gateway documentation for setup.

Use this skill when working with Jira Cloud through `aai-cli jira`.

Before running commands, confirm the active profile or pass `--profile`. Prefer typed commands for common issue, idea, sprint, board, project, comment, and attachment workflows; use `jira request` only for uncommon Jira REST endpoints.

For Jira Product Discovery, use `jira ideas` commands: they scope searches to Product Discovery projects, default the create issue type to `Idea`, and discover project-specific idea fields (`jira ideas fields PROJECT`) so custom fields like Impact or Effort can be set through `--json`.

When the user supplies a Product Discovery URL, run `jira ideas parse-url URL` before choosing a command. A `/jira/polaris/projects/PROJECT/ideas/view/VIEW_ID` URL identifies a saved view and project, not an idea; only its `selectedIssue` query parameter identifies an existing idea. Preserve the user's requested operation: a view URL plus “create” means create in the parsed project, while an issue key plus “update” means update that issue. Never pass a Product Discovery view id to `ideas get` or `ideas update`, and never create a replacement merely because an existing target cannot be resolved.

Jira workflow status changes use `jira ideas transitions list IDEA` followed by `jira ideas transitions perform IDEA --transition ID_OR_NAME`. Ordinary `ideas update` is for editable fields, not workflow transitions.

List commands aggregate Jira pagination up to `--limit` and preserve the documented response shape. Create and update commands accept typed flags and, where documented, `--json` payloads.

Successful output is JSON on stdout. Errors are structured JSON on stderr. See [the command reference](references/command-reference.md) for filters, payload rules, response shapes, and examples.
