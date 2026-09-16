# Auth Matrix

This project primarily consumes credentials supplied by users or agents. Microsoft delegated profiles additionally support an explicit one-time device bootstrap because unattended runs require a securely persisted refresh token.

## Atlassian Cloud

- Personal API token: Common for Jira and Confluence Cloud basic auth. A profile needs site URL, account email, and API token. Treat this as user-delegated and permissioned like the user.
- OAuth 2.0 3LO: User-delegated OAuth flow with scopes. Useful later for marketplace-style apps or delegated user access without storing API tokens.
- App/service-style access: Atlassian Cloud app models use app installation context and scoped permissions. This differs from a human API token and should be modeled separately when implemented.
- Bitbucket API/personal tokens: Bitbucket Cloud REST APIs use Basic auth with the Atlassian account email as the username and the API token as the password. Keep this separate from repository/workspace access tokens, which use bearer auth.

## GitHub

- Classic or fine-grained personal access token: User-owned token. Fine-grained PATs have endpoint-specific support and repository/resource restrictions.
- GitHub App installation token: Best service-account equivalent for automation. It is scoped to app permissions and installations rather than a human user.
- OAuth app token: User-delegated token for OAuth apps. Model separately from GitHub App installation tokens because renewal, scopes, and ownership differ.

## Google Workspace

- User OAuth token: Delegated user access for Gmail, Calendar, Sheets, and Drive. Scopes determine whether the CLI can read, send, or modify resources.
- Drive scopes: `drive.readonly` covers every implemented Drive command except `files upload`, which needs `drive.file` (files the app itself created) or full `drive`. `drive.metadata.readonly` is enough for `files list`/`files get`/`folders`/`drives`/`permissions` but **not** for `files download` — metadata scopes cannot fetch content. Drive also enforces its own per-file rule for `permissions.list`: a plain reader gets `403 insufficientFilePermissions` there even with a sufficient scope, so treat that 403 as file access, not token scope.
- User OAuth token: Delegated user access for Gmail and Calendar. Scopes determine whether the CLI can read, send, or modify resources.
- Service account: Server-to-server identity. For Gmail and user calendars in a Workspace domain, service accounts generally require domain-wide delegation plus user impersonation.
- API key: Not sufficient for private Gmail/Calendar user data and should not be used for the planned operations.

## Zoho

- User OAuth token: Primary model for Zoho Mail and Calendar APIs. Access tokens are short-lived and refresh tokens are needed for long-running use.
- Organization/admin access: Some Zoho APIs expose admin-level operations depending on product and scopes. Do not assume a generic service account model across Zoho products.
- Region-specific accounts: Zoho accounts and API hosts vary by region/data center, so profiles should include an accounts/base URL when needed.

## Apollo

- User API key: Primary implemented model. Profiles use `auth_type = "apollo_api_key"` with `api_token_secret`, and requests send the key in the `x-api-key` header.
- Key scope and plan access: Apollo API keys can be limited by endpoint access and account plan. Treat `403` as insufficient key scope, missing plan access, or a master-key-only endpoint.
- Partner OAuth: Apollo documents OAuth bearer tokens for partner integrations, but this CLI does not implement Apollo OAuth acquisition or refresh in this pass.

## HubSpot

- Service key: Implemented as `auth_type = "hubspot_service_key"` with `token_secret`. It authenticates HubSpot REST API requests, but service keys are not a replacement for developer-platform authentication such as webhooks or UI extensions.
- Legacy private app token: Implemented as `auth_type = "hubspot_legacy_private_app"` with `token_secret`. It uses the same bearer header shape but must remain distinct because some endpoint families have different support.
- Scope and tier access: HubSpot `401` and `403` responses should be returned as structured `auth_error` JSON with `details.provider`, `details.auth_type`, `details.endpoint`, `details.required_scopes`, and `details.remediation`. Do not pretend the CLI can validate Enterprise/account-tier entitlement locally.
- Custom channels: HubSpot documents conversations custom channel endpoints as unsupported for legacy private apps. The CLI should return `unsupported_auth` before the request when `hubspot_legacy_private_app` is used for `conversations custom-channels`.
- Common non-CRM scopes: files need `files`; hidden/deleted file reads may need `files.ui_hidden.read`; event occurrence reads need `business-intelligence`; custom behavioral event sends need `analytics.behavioral_events.send`; conversations reads need `conversations.read`; conversations writes usually need `conversations.write`; visitor identification token creation needs `conversations.visitor_identification.tokens.create`.

## Slack

- Bot token (`xoxb-`): Primary and only implemented model. Profiles use `auth_type = "bearer_token"` with `token_secret`, sent as a standard `Authorization: Bearer` header — no Slack-specific auth code was needed since `bearer_token` is already this CLI's default auth branch.
- Required scopes: `channels:read`, `groups:read` (channel metadata/listing, including private channels), `channels:history`, `groups:history` (message history, used internally by `links list`), `files:read`, `bookmarks:read`. `canvases:read` is not required by this CLI's canvas download, which uses `files.info`/`url_private_download` rather than the canvas-specific API (which has no content-read method — see [docs/services/slack.md](services/slack.md)).
- Internal apps only: this CLI assumes the Slack app/bot is installed only in its own workspace and never has public distribution enabled. Slack throttles `conversations.history`/`conversations.replies` to 1 request/minute for apps commercially distributed outside the Marketplace; internal apps keep normal Tier 2/3 limits ([details](https://docs.slack.dev/changelog/2025/06/03/rate-limits-clarity/)).
- OAuth install flow: not implemented. This CLI does not acquire or refresh Slack OAuth tokens — a bot token must be created and supplied by the user, same as every other provider in this matrix.

## Openpanel

- Client ID/secret (`auth_type = "openpanel_client_credentials"`): the only implemented model, sent as `openpanel-client-id` and `openpanel-client-secret` headers. Set `client_id` (not a secret) and `api_token_secret` (the client secret) on the profile.
- Client type matters: OpenPanel clients are typed `write`, `read`, or `root` server-side. `events export`, `insights *`, and `profiles *` all need a `read` or `root` client; `projects list`/`get` (the Manage API) needs a `root` client specifically. A `write` client (the default created with a new project) fails every command in this integration with `401` (verified live — the provider returns 401, not 403, for both the Export/Insights and Manage refusals).
- Optional `project_id` profile field: `insights *` and `profiles *` require a project ID in the URL; set `profile.project_id` to avoid passing `--project-id` on every call.
- OAuth install flow: not applicable — OpenPanel has no OAuth flow for the REST API, only static client credentials created in the dashboard under Settings → Clients.

## CLI Implications

- Store auth type explicitly in each profile: `basic_api_token`, `bearer_token`, `apollo_api_key`, `hubspot_service_key`, `hubspot_legacy_private_app`, `openpanel_client_credentials`, `github_app`, `oauth_user`, or `service_account`.
- Never infer service-account semantics from a token string alone.
- Keep provider profiles isolated; do not reuse an Atlassian token across Jira, Confluence, and Bitbucket unless the provider docs explicitly support it.
- Prefer env var overrides for secrets and config-file fields for non-secret metadata such as site URL, workspace, region, account email, and default scopes.
## Microsoft Graph

- `microsoft_client_credentials` uses `tenant_id`, `client_id`, and `client_secret_secret`. Each command obtains an app-only token for `https://graph.microsoft.com/.default` without user interaction.
- `microsoft_delegated` uses `tenant_id`, `client_id`, `scope`, and `refresh_token_secret`. Run `microsoft auth login` once; it requires `offline_access`, validates `/me` against `profile.user_id` when configured, and stores the refresh token encrypted.
- Refresh responses can rotate the delegated refresh token. The CLI replaces the encrypted value before returning the Graph response.
- `microsoft auth status` is noninteractive. A revoked or expired delegated credential fails with an instruction to run `microsoft auth login` again.
- Microsoft To Do commands require `microsoft_delegated`. Graph's application-permission support varies by To Do operation, so the typed surface uses one delegated identity for complete list/task CRUD and rejects app-only profiles before making a request.
- Microsoft Excel workbook commands require `microsoft_delegated` with `Files.ReadWrite`; Graph does not support application permissions for workbook operations and the CLI rejects app-only profiles before making a request.
- Outlook, SharePoint, Teams reads, and Planner commands support the permissions granted to the profile. Planner task updates and deletes require the last observed `@odata.etag` via `--etag`.
- Word files are transferred through OneDrive/SharePoint file commands only. The CLI does not semantically edit `.docx` content; agents must download, edit externally, and upload the complete replacement.
