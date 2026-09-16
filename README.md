# aai-cli

`aai-cli` is a Rust command-line toolkit that gives agents and automation a consistent JSON interface to common work systems. It wraps provider APIs with predictable commands, JSON stdout, and structured JSON errors on stderr.

The goal is not to replace full SDKs. The goal is to make common agent tasks safe and scriptable without each agent learning every provider API.

## Supported Services

| Service | CLI coverage | Local docs | Original API docs |
| --- | --- | --- | --- |
| Jira Cloud | Issues, Product Discovery ideas, URL parsing, field discovery and workflow transitions, projects, Agile boards, sprints, comments, attachments | [docs/services/jira.md](docs/services/jira.md) | [Jira REST API v3](https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/), [Jira Software REST API](https://developer.atlassian.com/cloud/jira/software/rest/intro/) |
| Confluence Cloud | Spaces, pages, comments, attachments, search, page moves | [docs/services/confluence.md](docs/services/confluence.md) | [Confluence REST API v2](https://developer.atlassian.com/cloud/confluence/rest/v2/intro/) |
| Bitbucket Cloud | Repositories, branches, commits, source files, pull requests, comments, pipelines | [docs/services/bitbucket.md](docs/services/bitbucket.md) | [Bitbucket REST API](https://developer.atlassian.com/cloud/bitbucket/rest/intro/) |
| GitHub | Repositories, branches, files, issues, pull requests, reviews, Actions runs/jobs/logs | [docs/services/github.md](docs/services/github.md) | [GitHub REST API](https://docs.github.com/en/rest) |
| Email | Gmail REST profiles and Zoho SMTP/IMAP profiles | [docs/services/email.md](docs/services/email.md) | [Gmail API](https://developers.google.com/gmail/api/reference/rest), [Zoho Mail API](https://www.zoho.com/mail/help/api/) |
| Calendar | Google Calendar REST profiles and Zoho CalDAV profiles | [docs/services/calendar.md](docs/services/calendar.md) | [Google Calendar API](https://developers.google.com/calendar/api/v3/reference), [Zoho CalDAV](https://www.zoho.com/calendar/help/caldav-sync.html) |
| Pipedrive | Leads, persons, organizations, deals, labels, activities, notes, synced email history | [docs/services/pipedrive.md](docs/services/pipedrive.md) | [Pipedrive API](https://developers.pipedrive.com/docs/api/v1), [Pipedrive API v2](https://developers.pipedrive.com/docs/api/v2) |
| Apollo | People and organization search/enrichment, contacts, accounts, deals, tasks, calls, outreach, conversations, analytics | [docs/services/apollo.md](docs/services/apollo.md) | [Apollo API docs](https://docs.apollo.io/docs/apollo-api-overview), [Apollo OpenAPI](https://docs.apollo.io/openapi/apollo-rest-api.json) |
| HubSpot | CRM object reads/search, files, events, conversations, visitor identification, custom channels | [docs/services/hubspot.md](docs/services/hubspot.md) | [HubSpot API docs](https://developers.hubspot.com/docs/api/overview) |
| Google Sheets | Spreadsheet listing plus sheet value reads/writes | [docs/services/sheets.md](docs/services/sheets.md) | [Google Sheets API](https://developers.google.com/sheets/api/reference/rest) |
| Google Drive | Files and folders, blob and Google-native content reads, shared drives, sharing (read-only), storage quota, plus file upload | [docs/services/drive.md](docs/services/drive.md) | [Google Drive API v3](https://developers.google.com/workspace/drive/api/reference/rest/v3) |
| Slack | Channel metadata, files, bookmarks, links, channel canvas download (read-only, bot token) | [docs/services/slack.md](docs/services/slack.md) | [Slack Web API](https://docs.slack.dev/reference/methods) |
| Slack | Channel metadata, files, bookmarks, links, channel canvas download (read-only, bot token) | [docs/services/slack.md](docs/services/slack.md) | [Slack Web API](https://docs.slack.dev/reference/methods) |
| OpenPanel | Projects, raw event export, insights (metrics, pages, referrers, devices, geo), profiles (read-only, client ID/secret) | [docs/services/openpanel.md](docs/services/openpanel.md) | [OpenPanel API docs](https://openpanel.dev/docs/api) |
| Microsoft Graph | Durable app-only and delegated authentication; Outlook mail/calendar/contacts; OneDrive and SharePoint files/lists; Teams reads; To Do lists/tasks; Planner tasks; generic Graph requests | [docs/microsoft-e2e-setup.md](docs/microsoft-e2e-setup.md) | [Microsoft Graph REST API](https://learn.microsoft.com/en-us/graph/api/overview) |

Project features that are not provider services:

- Persistent profile inspection, editing, validation, and default-profile management.
- Local encrypted secrets using XChaCha20-Poly1305.
- Bundled Agent Skills that can be discovered, validated, and installed into an active skills directory.
- Generic authenticated `request` commands for HTTP-backed services.

## Documentation

- [Command reference](docs/aai-cli-command-reference.md): implemented CLI commands, flags, pagination behavior, and agent usage notes.
- [Auth matrix](docs/auth-matrix.md): supported credential models and provider-specific auth notes.
- [API documentation snapshot](docs/README.md): local machine-readable specs and saved provider docs.
- [Token refresh notes](docs/token-refresh.md): OAuth token refresh behavior for supported REST profiles.
- [Microsoft Graph setup and live tests](docs/microsoft-e2e-setup.md): idempotent tenant provisioning, durable login, resource checks, and behavioral verification.
- [Microsoft Graph ADR](docs/adr/0001-microsoft-graph-integration.md): command structure, auth choices, alternatives, and compatibility consequences.
- [Changelog](CHANGELOG.md): user-visible additions and changes.
- [Apollo implementation notes](docs/apollo/api-client.md): Apollo-specific base URLs, auth, rate limits, and endpoint coverage.

## Quick Start

Install from the checked-out repository:

```bash
cargo install --path .
```

Build and test without installing:

```bash
cargo build
scripts/run-tests.sh safe
```

Run through Cargo:

```bash
cargo run -- --profile jira-work jira issues list
```

Run an installed binary:

```bash
aai-cli --profile github-work github issues list
```

All successful command output is JSON. Errors are JSON on stderr and include `code`, `service`, `operation`, `status`, and `details`. Successful service responses include `_aai.pagination` with pagination status and instructions.

## Configuration

Default config path:

```text
~/.config/aai-cli/config.toml
```

Override config and profile:

```bash
aai-cli --config local/e2e.config.toml --profile jira-work jira projects list
```

Equivalent env vars:

```bash
AAI_CONFIG=local/e2e.config.toml
AAI_PROFILE=jira-work
```

Profiles should reference secrets instead of embedding credentials:

```toml
[profiles.github-work]
provider = "github"
auth_type = "bearer_token"
token_secret = "github.token"
owner = "acme"
repo = "app"
```

HubSpot profiles use explicit bearer auth models:

```toml
[profiles.hubspot-work]
provider = "hubspot"
auth_type = "hubspot_service_key" # or "hubspot_legacy_private_app"
token_secret = "hubspot.token"
```

Store secret values with:

```bash
printf '%s' "$GITHUB_TOKEN" | aai-cli --config local/e2e.config.toml secrets set github.token
```

Keep credentials and generated local configs under ignored `local/`; never commit them.

## Bundled Agent Skills

`aai-cli` embeds Agent Skills for supported provider workflows. They are inert package assets until installed:

```bash
aai-cli skills discover
aai-cli skills validate
aai-cli skills install aai-github
aai-cli skills install --all
```

Skills install to `~/.agents/skills/` by default. Use `--target-dir PATH` for another client directory, `--dry-run` to preview writes, and `--force` to overwrite an existing installed skill.

## Generic Authenticated Requests

Typed commands are preferred for common workflows. For reports and uncommon provider endpoints, HTTP-backed services expose an authenticated `request` escape hatch:

```bash
aai-cli --profile github-work github request get /repos/acme/app/issues --query state=closed
aai-cli --profile hubspot-work hubspot request get /crm/v3/objects/contacts
aai-cli --profile apollo-work apollo request get /users/api_profile
aai-cli --profile jira-work jira request post /rest/api/3/issue --allow-write --json -
aai-cli --profile microsoft-e2e-delegated microsoft request get /me
```

Supported services are `jira`, `confluence`, `bitbucket`, `github`, `hubspot`, `pipedrive`, `apollo`, `microsoft`, and REST-backed `email` and `calendar` profiles. SMTP/IMAP and CalDAV profiles are intentionally excluded.

## Development

Common commands:

```bash
cargo build
cargo check --all-targets --all-features
cargo fmt --check
cargo clippy --all-targets -- -D warnings
scripts/run-tests.sh safe
```

Live E2E tests are ignored by default and require local credentials:

```bash
AAI_E2E_CONFIG=./local/e2e.config.toml scripts/run-tests.sh live
```
