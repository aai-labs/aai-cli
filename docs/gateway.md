# Managed credential gateway

`aai-gateway` is a separate process for installations where the CLI user must not
receive provider API keys. Provider credentials are stored only by the gateway and are
injected into outbound requests after proxy-token and grant checks.

Gateway-mode CLI profiles send a typed CLI command to `/api/v2/execute`. The gateway
parses that command, constructs the provider request using its own service code and
stored profile, and checks the actual operation and HTTP method before sending it.
Clients cannot supply a provider URL or relabel an HTTP request as a different
operation. The older URL-based `/api/v1/execute` endpoint returns `410 Gone`.
The gateway returns command data and structured errors, not the outbound request
or its authorization headers; upstream response headers are not forwarded.

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

Input and output files are transferred with the command and are limited to 64 MiB
each. The gateway writes uploads into temporary storage and returns downloads to
the CLI for writing on the client machine. Google Drive resumable-upload session
URLs remain on the gateway. The server rejects provider redirects outside its
configured or service-specific destination allowlist. Configure the HTTPS reverse
proxy to accept multipart requests of at least 70 MiB for the 64 MiB file limit.

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

## Live parity checks

With `AAI_E2E_CONFIG` pointing at a local config whose GitHub and Bitbucket test
credentials resolve, run the ignored read-only comparisons separately:

```bash
cargo test --bin aai-cli live_saved_github_credentials_match_direct_and_gateway_results -- --ignored
cargo test --bin aai-cli live_saved_bitbucket_credentials_match_direct_and_gateway_results -- --ignored
```

Each check uses a temporary loopback gateway and encrypted state, then compares its
result with a direct call. The tests do not print provider data or credentials.
