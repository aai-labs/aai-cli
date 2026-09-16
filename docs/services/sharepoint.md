# SharePoint

## CLI Scope

SharePoint commands cover site resolution, document libraries (drives), drive items, file download and upload, and drive change tracking via `delta`, through Microsoft Graph v1.0. Authentication is **app-only**: commands act as the organisation's own Entra app, which reaches only the sites its administrator granted. Use the full command reference for exact flags:

- [aai-cli command reference](../aai-cli-command-reference.md#sharepoint)
- [Auth matrix](../auth-matrix.md#microsoft-365--sharepoint)

## Setup

1. In the [Microsoft Entra admin center](https://entra.microsoft.com), register an application (single-tenant is fine).
2. Under **Certificates & secrets**, create a client secret and copy its value.
3. Under **API permissions**, add the Microsoft Graph **application** permission `Sites.Selected`, then **Grant admin consent**.
4. Grant the app each site it should reach. There is no admin-center UI for this; use one of:

   ```bash
   # CLI for Microsoft 365
   m365 spo site apppermission add --appId <app-id> \
     --siteUrl https://contoso.sharepoint.com/sites/project-x --permission write
   ```

   ```powershell
   # PnP PowerShell (needs its own app registration to sign in since September 2024)
   Grant-PnPEntraIDAppSitePermission -AppId <app-id> -DisplayName "Agent" `
     -Site https://contoso.sharepoint.com/sites/project-x -Permissions Write
   ```

   or `POST https://graph.microsoft.com/v1.0/sites/{site-id}/permissions` in Graph Explorer. Roles are `read`, `write`, `manage` and `fullcontrol`; uploads need `write`.

5. Configure the profile with the tenant id, client id and the client secret reference.

## Original API Docs

- [Microsoft Graph REST API v1.0 reference](https://learn.microsoft.com/en-us/graph/api/overview)
- [sites resource type](https://learn.microsoft.com/en-us/graph/api/resources/site)
- [drive resource type](https://learn.microsoft.com/en-us/graph/api/resources/drive)
- [driveItem resource type](https://learn.microsoft.com/en-us/graph/api/resources/driveitem)
- [Create site permission (Sites.Selected grants)](https://learn.microsoft.com/en-us/graph/api/site-post-permissions)
- [Track changes with delta](https://learn.microsoft.com/en-us/graph/api/driveitem-delta)
- [Upload small files](https://learn.microsoft.com/en-us/graph/api/driveitem-put-content)
- [Microsoft Graph throttling guidance](https://learn.microsoft.com/en-us/graph/throttling)

## Implementation Notes

Things that are not obvious from the docs alone:

- **`Sites.Selected` grants nothing by itself.** It makes the app eligible; every site must then be granted individually. An app with the permission but no grants gets 403 on everything.
- **Granted sites cannot be enumerated.** `GET /sites?search=*` returns 403 under `Sites.Selected`, so callers must be told which site URLs to use.
- **Client secrets are fine for Graph, not for SharePoint REST.** SharePoint's own `_api` endpoints reject app-only tokens issued for a client secret and require a certificate. These commands use Graph only.
- **Sites are addressed two ways.** Graph accepts an opaque composite id (`host,siteGuid,webGuid`) *and* a `{hostname}:/{server-relative-path}` form. The CLI converts a pasted SharePoint URL into the latter and passes anything else through untouched.
- **Collections are wrapped in `value`, not `values`.** Graph's singular key had to be added to the shared pagination collection keys.
- **`@odata.nextLink` and `@odata.deltaLink` are different things.** `nextLink` is another page of the current result; `deltaLink` is the cursor for the *next sync run*. Only the former is treated as a continuation.
- **Item listings carry a large signed download URL.** `@microsoft.graph.downloadUrl` is dropped when trimming, since it is short-lived and bulky; `items download` re-derives it through `/content`.
- **Graph throttles hard**, returning `429` with `Retry-After`, and `503` during service blips. The shared HTTP layer retries both.
