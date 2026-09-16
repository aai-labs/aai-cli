# API Documentation Snapshot

This directory contains official provider documentation used to implement `aai-cli`.
Run `bash scripts/fetch-api-docs.sh` from the repository root to refresh the snapshot.

## Machine-Readable Specs

| Provider | Service | Local file |
| --- | --- | --- |
| Atlassian | Jira Cloud | `docs/atlassian/jira/openapi.json` |
| Atlassian | Confluence Cloud | `docs/atlassian/confluence/openapi.json` |
| Atlassian | Bitbucket Cloud | `docs/atlassian/bitbucket/openapi.json` |
| GitHub | REST API | `docs/github/rest-api-openapi.json` |
| Google | Gmail API | `docs/google/gmail/discovery-v1.json` |
| Google | Calendar API | `docs/google/calendar/discovery-v3.json` |
| Pipedrive | CRM API v1 | `docs/pipedrive/openapi-v1.json` |
| Pipedrive | CRM API v2 | `docs/pipedrive/openapi-v2.json` |

## Developer Guides

Service-level CLI docs live under `docs/services/`:

- [Jira](services/jira.md)
- [Confluence](services/confluence.md)
- [Bitbucket](services/bitbucket.md)
- [GitHub](services/github.md)
- [Email](services/email.md)
- [Calendar](services/calendar.md)
- [Pipedrive](services/pipedrive.md)
- [Apollo](services/apollo.md)
- [Google Sheets](services/sheets.md)
- [Google Drive](services/drive.md)
- [OpenPanel](services/openpanel.md)
- [Microsoft Graph](microsoft-e2e-setup.md)

Atlassian docs include REST entrypoints, auth/API-token docs, OAuth scopes, webhooks, rate limits, and Jira ADF references. Start with:

- `docs/atlassian/jira/rest-v3-intro.html`
- `docs/atlassian/jira/adf-structure.html`
- `docs/atlassian/jira/adf-schema.json`
- `docs/atlassian/confluence/rest-v2-intro.html`
- `docs/atlassian/bitbucket/rest-index.html`

GitHub docs include REST authentication, GitHub App authentication, fine-grained PAT support, pagination, rate limits, best practices, and webhook payloads.

Google docs include Gmail and Calendar discovery docs, REST references, scopes, sync, push notifications, batch guidance, service accounts, and domain-wide delegation.

Zoho docs include official Mail and Calendar HTML API references. Zoho does not appear to publish official OpenAPI specs for these products, so the saved docs are HTML pages plus the manifest.

Pipedrive docs include official v1 and v2 OpenAPI specs, the API reference, personal API token guide, and v2 overview. The CLI uses v1 for leads/lead labels and v2 for deals, persons, organizations, and search where available.

Apollo docs include an implementation summary for auth, base URLs, rate limits, credits, endpoint groups, and OpenAPI usage in `docs/apollo/api-client.md`.

## Local Reference Files

- `docs/aai-cli-command-reference.md` is the agent-facing command reference for implemented CLI behavior, including Jira/Confluence search, pagination, Confluence page moves, GitHub Actions status/logs, and Bitbucket Pipelines status/logs.
- `docs/manifest.json` records every source URL, output path, format, size, status, and retrieval timestamp.
- `docs/auth-matrix.md` summarizes personal token, service account, app token, and OAuth differences across providers.
- `docs/microsoft-e2e-setup.md` documents repeatable Microsoft 365 provisioning, durable delegated login, resource verification, and live behavioral tests.
- `docs/adr/0001-microsoft-graph-integration.md` records the Microsoft Graph command/auth architecture and its compatibility consequences.
- `docs/drive-command-test-results.md` is the live-run report for the `aai-cli drive` command surface. Regenerate it with `scripts/drive-command-report.sh`.
- `docs/atlassian/adf.md` summarizes the Atlassian Document Format files needed for Jira/Confluence implementation.
