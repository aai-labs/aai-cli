# Microsoft Graph live E2E setup

This runbook provisions the isolated Microsoft 365 resources used by the AAI CLI behavioral live tests. The setup is repeatable: subsequent runs find the resources by their fixed names, repair memberships and permissions, and rewrite the local ID file.

The provisioned workspace contains:

- One confidential Entra application for application-permission tests.
- One public-client Entra application for delegated tests.
- One existing licensed test user.
- One private Team and standard channel.
- The Team's associated SharePoint site and default document library.
- One SharePoint list with `Status` and `ExternalId` columns.
- One Planner plan and bucket in the Team's Microsoft 365 group.

The script writes non-secret IDs to ignored `local/e2e-ms.env`. It writes the confidential application's secret directly to the CLI's encrypted local secret store. It never writes the secret to the environment file.

## Prerequisites

Use PowerShell 7.2 or newer. Install PowerShell using Microsoft's instructions for your operating system, then install the Microsoft Graph SDK:

```powershell
Install-Module Microsoft.Graph -Scope CurrentUser
```

Alternatively, let the provisioning script install missing Graph submodules by passing `-InstallModules`. Existing modules are reused and are not reinstalled or forcibly upgraded.

The interactive provisioning account needs an Entra role capable of creating applications and granting admin consent, plus sufficient Teams, SharePoint, and Microsoft 365 group administration privileges. The script requests these delegated scopes:

```text
Application.ReadWrite.All
AppRoleAssignment.ReadWrite.All
DelegatedPermissionGrant.ReadWrite.All
Directory.Read.All
User.Read.All
Group.ReadWrite.All
Channel.Create
Sites.FullControl.All
Tasks.ReadWrite
```

Use a non-production tenant. The E2E application permissions intentionally include tenant-wide mailbox, file, and collaboration reads/writes so the test suite can exercise both application and delegated behavior. Do not deploy these credentials into a customer or production environment.

The test user must already have licenses that enable Exchange Online, SharePoint/OneDrive, Teams, Planner, and Microsoft To Do. Verify that user before provisioning:

```powershell
Connect-MgGraph -Scopes "User.Read.All", "Organization.Read.All"
Get-MgUserLicenseDetail -UserId "aai-cli-e2e@example.onmicrosoft.com" |
    Select-Object SkuPartNumber, SkuId
Disconnect-MgGraph
```

License assignment is deliberately outside the script because available SKU names and service-plan combinations vary by tenant.

## Provision or repair the workspace

Run from the repository root:

```powershell
./scripts/ms-e2e.ps1 `
    -Mode provision `
    -TestUserUpn "aai-cli-e2e@example.onmicrosoft.com" `
    -InstallModules
```

On the first run, Microsoft prints a device-login URL and code. Open the URL in any browser, enter the code, and authenticate as the provisioning administrator. Authentication is cached for the current operating-system user, so later runs normally reuse it. Team and SharePoint provisioning are asynchronous; the script retries each for up to 15 minutes.

On Unix, the script disables IPv6 for its own PowerShell process. This avoids authentication stalls on hosts that advertise IPv6 DNS results but do not have a working IPv6 route. It does not change operating-system networking.

Every `provision` run:

1. Creates or updates the two app registrations and enterprise applications.
2. Repairs application permission assignments and delegated admin consent.
3. Reuses the app-only credential when both the Entra credential and encrypted local secret exist. If either side is missing or ambiguous, it safely creates and stores a replacement before removing stale script-owned credentials.
4. Creates or reuses the Team, channel, SharePoint site, document library, list, Planner plan, and bucket.
5. Writes current resource identifiers to `local/e2e-ms.env`.

Do not run simultaneous provisioning sessions with the same `-ResourceName`.

## Rediscover IDs without rotating the secret

Use `discover` after cloning onto another workstation or after deleting the generated environment file:

```powershell
./scripts/ms-e2e.ps1 `
    -Mode discover `
    -TestUserUpn "aai-cli-e2e@example.onmicrosoft.com"
```

Discovery reuses the existing applications and resources and recreates missing collaboration resources. It does not grant permissions or rotate the app secret.

## Load the generated variables

PowerShell:

```powershell
Get-Content local/e2e-ms.env | ForEach-Object {
    if ($_ -match '^([^#][^=]+)=(.*)$') {
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], "Process")
    }
}
$env:AAI_E2E_CONFIG = "./local/e2e.config.toml"
```

Bash:

```bash
set -a
. ./local/e2e-ms.env
set +a
export AAI_E2E_CONFIG=./local/e2e.config.toml
```

The generated profile values are stable local names:

```text
AAI_E2E_MS_APP_PROFILE=microsoft-e2e-app
AAI_E2E_MS_DELEGATED_PROFILE=microsoft-e2e-delegated
```

Provisioning also creates or updates both profiles in `local/e2e.config.toml`. They reference:

- `$AAI_E2E_MS_TENANT_ID`
- `$AAI_E2E_MS_APP_CLIENT_ID`
- Encrypted secret key `microsoft.e2e.app.client_secret`
- `$AAI_E2E_MS_DELEGATED_CLIENT_ID`
- Encrypted secret key `microsoft.e2e.delegated.refresh_token`

The app profile acquires Graph tokens noninteractively with client credentials. The delegated profile uses an encrypted refresh token after the one-time bootstrap below.

## Bootstrap the durable delegated session

Run this once as the E2E user:

```bash
cargo run --quiet -- \
  --config local/e2e.config.toml \
  --profile microsoft-e2e-delegated \
  microsoft auth login
```

The command requests `offline_access`, verifies that `/me` matches the configured `user_id`, and writes the refresh token directly to the encrypted secret store. It never writes the token to TOML or the environment file.

Normal checks are noninteractive. These commands acquire fresh access tokens solely from saved credentials:

```bash
cargo run --quiet -- \
  --config local/e2e.config.toml \
  --profile microsoft-e2e-app \
  microsoft auth status

cargo run --quiet -- \
  --config local/e2e.config.toml \
  --profile microsoft-e2e-delegated \
  microsoft auth status
```

Microsoft can return a replacement refresh token during every refresh. The CLI atomically replaces the encrypted value so later processes keep using the newest credential. If Microsoft revokes or expires the session, `auth status` fails with an instruction to rerun `microsoft auth login`.

## Verify the provisioned resources

The read-only checker uses only the saved app secret and delegated refresh token:

```powershell
./scripts/ms-e2e-check.ps1
```

It verifies both saved credentials, the expected delegated identity, the test user, direct Team membership, Team/channel, SharePoint site/drive/list, and Planner plan/bucket relationships. It never calls `Connect-MgGraph`, prompts for login, or creates data.

## Run the behavioral live suite

Load the generated environment and run the ignored live test:

```bash
set -a
. ./local/e2e-ms.env
set +a
export AAI_E2E_CONFIG=./local/e2e.config.toml
cargo test --test microsoft_e2e_live -- --ignored --nocapture --test-threads=1
```

The three Given/When/Then tests exercise complete, isolated flows:

- Outlook draft, event, and contact create/read/update/delete, plus collection reads.
- Mail send to the test user, inbox/sent verification, and deletion of both copies.
- SharePoint list-item create/read/update/delete.
- Microsoft To Do list/task and Planner task create/read/update/delete.
- Team, channel, team-member, channel-message, and chat reads.
- A minimal Word document uploaded to OneDrive, downloaded and inspected, uploaded to SharePoint, downloaded and inspected again, then deleted from both drives.

Every created resource has a best-effort cleanup guard so an assertion failure does not normally leave test data behind. The tests also explicitly verify deletion where Graph supports an immediate read-after-delete check. Run serially because they share one mailbox and collaboration workspace.

## Remove the workspace

Preview the destructive operation:

```powershell
./scripts/ms-e2e.ps1 `
    -Mode remove `
    -TestUserUpn "aai-cli-e2e@example.onmicrosoft.com" `
    -WhatIf
```

Remove the Team-backed Microsoft 365 group. This also schedules removal of its Team, SharePoint site, list, files, and Planner data:

```powershell
./scripts/ms-e2e.ps1 `
    -Mode remove `
    -TestUserUpn "aai-cli-e2e@example.onmicrosoft.com" `
    -Confirm
```

App registrations are retained by default so a resource reset does not disturb credentials. Include `-RemoveApplications` to remove them too:

```powershell
./scripts/ms-e2e.ps1 `
    -Mode remove `
    -TestUserUpn "aai-cli-e2e@example.onmicrosoft.com" `
    -RemoveApplications `
    -Confirm
```

After removal, delete the ignored generated ID file. Remove the encrypted secret only if the application registration was also removed:

```powershell
Remove-Item local/e2e-ms.env

cargo run --quiet -- `
    --secrets-file local/aai-secrets.enc.json `
    --key-file local/aai-secrets.key `
    secrets remove microsoft.e2e.app.client_secret
```

## Common failures

- `Authorization_RequestDenied`: the signed-in administrator lacks the required Entra role or Graph consent permissions.
- Team conversion returns `404`: Microsoft 365 group replication is still in progress. The script retries; rerun it if the 15-minute limit expires.
- SharePoint site is not found: Team-backed site provisioning is still in progress. Rerun in `discover` mode later.
- Planner returns `403`: the signed-in provisioning account is not a group member, or Planner is disabled for that account. A `provision` rerun repairs group membership.
- A Graph permission cannot be resolved: the permission name changed or is unavailable in the tenant/cloud. The script fails before granting an incomplete permission set.
- More than one resource has the fixed name: rename or remove the duplicate. The script refuses to choose one implicitly.

## Microsoft references

- [Install the Microsoft Graph PowerShell SDK](https://learn.microsoft.com/en-us/graph/sdks/sdk-installation)
- [Microsoft Graph PowerShell authentication](https://learn.microsoft.com/en-us/powershell/microsoftgraph/authentication-commands?view=graph-powershell-1.0)
- [Grant application permissions](https://learn.microsoft.com/en-us/powershell/microsoftgraph/how-to-grant-revoke-api-permissions?view=graph-powershell-1.0)
- [Create delegated permission grants](https://learn.microsoft.com/en-us/graph/api/oauth2permissiongrant-post?view=graph-rest-1.0)
- [Create a Team from a Microsoft 365 group](https://learn.microsoft.com/en-us/graph/api/team-put-teams?view=graph-rest-1.0)
- [Access a group's SharePoint site](https://learn.microsoft.com/en-us/graph/api/resources/site?view=graph-rest-1.0)
- [Create a SharePoint list](https://learn.microsoft.com/en-us/graph/api/list-create?view=graph-rest-1.0)
- [Create a Planner plan](https://learn.microsoft.com/en-us/graph/api/planner-post-plans?view=graph-rest-1.0)
