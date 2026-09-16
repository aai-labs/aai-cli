---
name: aai-microsoft
description: Use aai-cli with durable Microsoft Graph credentials for Outlook mail, calendar and contacts; OneDrive and SharePoint files and lists; Teams reads; Microsoft To Do; and Planner tasks.
---

# aai-cli Microsoft Graph

Use this skill for Microsoft 365 work through `aai-cli microsoft`.

Confirm the active profile or pass `--profile`. App-only profiles are best for unattended organization-owned automation. Delegated profiles act as one user and are required for Microsoft To Do. Both obtain short-lived access tokens automatically from credentials saved in the encrypted secret store; never request or copy an access token into a command.

Prefer typed commands for supported operations. Use `microsoft request` only for a Graph endpoint without a typed command. Writes through `request` require `--allow-write`.

Most user resources accept `--user-id`; otherwise they use `profile.user_id`. SharePoint list commands require site and list IDs. Use `microsoft files` for OneDrive and the explicit `microsoft sharepoint files` commands with `--drive-id` for a SharePoint document library. Planner task update/delete require the current `@odata.etag` from a preceding get/create/update response.

Treat creates, sends, updates, and deletes as external side effects. Read the target first when practical, use stable identifiers, and report what changed. Supply request bodies through `--json PATH` or `--json -` for complex or sensitive values rather than shell-inline JSON.

List commands aggregate Graph pages up to `--limit` and return the provider's `value` array. Successful output is JSON on stdout; errors are structured JSON on stderr. Downloaded file bytes go only to `--output`.

See [the command reference](references/command-reference.md) for command shapes, auth constraints, common bodies, and safe workflows.
