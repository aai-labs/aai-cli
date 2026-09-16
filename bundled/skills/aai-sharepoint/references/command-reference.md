# aai-cli SharePoint Skill

Agent reference for the `aai-cli sharepoint` command group, backed by Microsoft Graph v1.0.

## Global flags

Accepted by every command. Can also be set via environment variables.

| Flag | Env | Default | Description |
|---|---|---|---|
| `--profile NAME` | `AAI_PROFILE` | config `default_profile` | Profile from `~/.config/aai-cli/config.toml` |
| `--config PATH` | `AAI_CONFIG` | `~/.config/aai-cli/config.toml` | Path to config file |
| `--secrets-file PATH` | `AAI_SECRETS_FILE` | `~/.config/aai-cli/secrets.enc.json` | Path to encrypted secrets file |
| `--key-file PATH` | `AAI_SECRET_KEY_FILE` | `/run/aai/key` or `~/.config/aai-cli/key` | Path to decryption key file |

## Profile & Authentication

SharePoint profiles authenticate as the organisation's own Microsoft Entra app using the
client-credentials grant. There is no user and no refresh token: the CLI requests a fresh
app-only token on each run.

```toml
[profiles.sharepoint-work]
provider = "sharepoint"
auth_type = "microsoft_client_credentials"
tenant_id = "contoso.onmicrosoft.com"
client_id = "00000000-0000-0000-0000-000000000000"
client_secret_secret = "sharepoint.client_secret"
```

- `tenant_id` is required — a tenant GUID or verified domain. `common` cannot issue app-only
  tokens.
- `base_url` overrides the Graph endpoint; it defaults to `https://graph.microsoft.com/v1.0`.
- `scope` optionally overrides the token scope; it defaults to
  `https://graph.microsoft.com/.default`.
- The client secret is referenced by name and read from the encrypted secret store — never
  write it directly into the config file.

The app reaches **only the sites its administrator granted** under the `Sites.Selected`
permission. Because of that, `sites list` cannot enumerate them and typically returns 403 —
work from the site URLs you were given. A 403 on a site means it was not granted; uploads
additionally need a `write` grant on that site.

## Commands

### Sites

```bash
aai-cli sharepoint sites list [--search TEXT] [--limit N]
aai-cli sharepoint sites get <site>
```

`<site>` accepts either a Graph site id (`host,siteGuid,webGuid`) or a SharePoint URL such
as `https://contoso.sharepoint.com/sites/engineering`. URLs are converted to Graph's
`{host}:/{path}` addressing automatically; ids are passed through untouched.

`sites list` requires a search term and defaults to `*`. Under `Sites.Selected` it cannot enumerate granted sites and normally returns 403.

### Drives

```bash
aai-cli sharepoint drives list <site>
```

Lists the document libraries of a site. The `id` of the drive you want is the `<drive-id>`
for every `items` command.

### Items

```bash
aai-cli sharepoint items list <drive-id> [--path FOLDER] [--limit N]
aai-cli sharepoint items get <drive-id> <item-id>
aai-cli sharepoint items download <drive-id> <item-id> --output FILE
aai-cli sharepoint items upload <drive-id> <path> --file FILE --allow-write
aai-cli sharepoint items delta <drive-id> [--token TOKEN]
```

- `--path` is relative to the drive root, e.g. `--path "Shared Documents/2026"`. Omit it to
  list the root. Spaces are handled; do not pre-encode the path.
- `items upload` takes the **destination path**, not an item id, and creates or replaces the
  file at that path. `--allow-write` is required because it overwrites remote content.
- `items delta` enumerates changes. The response carries `@odata.deltaLink`; keep its
  `token` query value and pass it as `--token` next run to get only what changed since.
  Deleted items appear with a `deleted` facet.

### Escape hatch

```bash
aai-cli sharepoint request get /sites/<id>/lists --query '$top=5'
aai-cli sharepoint request post /sites/<id>/lists --json '{...}' --allow-write
```

Calls any Graph endpoint with profile authentication for cases the typed commands do not
cover. Paths are relative to the Graph base URL; absolute URLs are rejected. Write methods
require `--allow-write`.

## Response shapes

List commands return Graph's envelope with the page-local `value` array trimmed to the
fields agents need. The envelope is preserved, so `@odata.nextLink` and `@odata.deltaLink`
survive. `get` commands return the provider object untrimmed.

Trimmed fields:

| Command | Kept |
|---|---|
| `sites list` | `id`, `name`, `displayName`, `webUrl`, `description`, `createdDateTime`, `lastModifiedDateTime` |
| `drives list` | `id`, `name`, `driveType`, `webUrl`, `description`, `lastModifiedDateTime` |
| `items list`, `items delta` | `id`, `name`, `size`, `webUrl`, `eTag`, `cTag`, `createdDateTime`, `lastModifiedDateTime`, `file`, `folder`, `parentReference`, `deleted` |

`@microsoft.graph.downloadUrl` is deliberately dropped from item listings: it is a large,
short-lived signed URL. Use `items download` instead.

Every response is wrapped with an `_aai.pagination` block. For Graph, `returned_count` comes
from the `value` array and `continuation.source` is `@odata.nextLink` when more pages exist.
`@odata.deltaLink` is **not** treated as a next page — it marks the cursor for the next sync
run, not more of the current result.

## Throttling

Graph throttles aggressively. The shared HTTP layer retries `429` and `503` automatically,
honouring `Retry-After` (capped at 60s) and falling back to exponential backoff, for up to
three retries. A throttling error that reaches the agent has already been retried.

## Errors

Errors are structured JSON on stderr with a `code`, `service`, `operation`, and — for
provider failures — the Graph error body.

| Situation | Code | Note |
|---|---|---|
| Profile lacks app credentials or tenant | `auth_error` | Set `tenant_id`, `client_id` and `client_secret_secret` |
| Site not granted to the app | `provider_api_error` (403) | An administrator must grant this site to the app |
| Site or item does not exist | `provider_api_error` (404) | Check the id came from a `list` command |
| Upload without `--allow-write` | `invalid_input` | Intentional guard on overwriting remote content |
