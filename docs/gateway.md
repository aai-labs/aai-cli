# Managed credential gateway

`aai-gateway` is a separate process for installations where the CLI user must not
receive provider API keys. Provider credentials are stored only by the gateway and are
injected into outbound requests after proxy-token and grant checks.

The gateway state file is encrypted with a separate 32-byte key file. Keep both files
on the gateway host and protect them with filesystem permissions. Put the gateway
behind an HTTPS reverse proxy before giving clients a remote URL.

## Grants

Each proxy token can be granted several remote profiles. A grant requires both an
allowed operation (or `allow_all_operations`) and an allowed HTTP method. Explicit
operation and method denies win. New grants allow nothing.

The gateway supports typed HTTP operations for Agent Barn, Apollo, Bitbucket,
Confluence, GitHub, Google Drive, Gmail, Google Sheets, HubSpot, Jira, Merkys,
Microsoft Graph, OpenPanel, Pipedrive, PostHog, Slack, and Zoho Mail. It does not
expose generic `request` or Custom API escape hatches. SMTP/IMAP, CalDAV, Agent Barn
stream capture, Microsoft interactive device login, and local Excel operations are
not gateway transports in this version.

## Administration

Set `AAI_GATEWAY_URL` and `AAI_GATEWAY_ADMIN_TOKEN` for the administrative CLI:

```bash
aai-cli gateway profiles list
aai-cli gateway profiles create --json profile.json
aai-cli gateway tokens create --json token-grant.json
aai-cli gateway tokens update TOKEN_ID --json revoke.json
```

Provider profile reads report metadata and secret-presence flags only. A newly created
proxy token is returned once; store it in the CLI's encrypted local secret store.
