#Requires -Version 7.2

[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [ValidateSet("provision", "discover", "remove")]
    [string] $Mode = "provision",

    [Parameter(Mandatory)]
    [string] $TestUserUpn,

    [string] $ResourceName = "AAI CLI E2E",
    [string] $EnvironmentPath = "local/e2e-ms.env",
    [string] $ConfigPath = "local/e2e.config.toml",
    [string] $SecretsFile = "local/aai-secrets.enc.json",
    [string] $KeyFile = "local/aai-secrets.key",
    [string] $AppSecretKey = "microsoft.e2e.app.client_secret",
    [string] $DelegatedRefreshTokenKey = "microsoft.e2e.delegated.refresh_token",
    [switch] $InstallModules,
    [switch] $RemoveApplications
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# Microsoft public endpoints support IPv4. Restrict only this PowerShell process on
# Unix hosts so a configured-but-unroutable IPv6 interface cannot stall .NET auth.
if (-not $IsWindows) {
    [System.AppContext]::SetSwitch("System.Net.DisableIPv6", $true)
}

$GraphAppId = "00000003-0000-0000-c000-000000000000"
$AppOnlyName = "$ResourceName - Application"
$DelegatedName = "$ResourceName - Delegated"
$ChannelName = "cli-e2e"
$ListName = "$ResourceName Items"
$PlanName = "$ResourceName Plan"
$BucketName = "E2E"

$ApplicationPermissions = @(
    "User.Read.All",
    "Group.Read.All",
    "Mail.ReadWrite",
    "Mail.Send",
    "Calendars.ReadWrite",
    "Contacts.ReadWrite",
    "Files.ReadWrite.All",
    "Sites.ReadWrite.All",
    "Team.ReadBasic.All",
    "Channel.ReadBasic.All",
    "TeamMember.Read.All",
    "ChannelMessage.Read.All",
    "Chat.Read.All",
    "Tasks.ReadWrite.All"
)

$DelegatedPermissions = @(
    "User.Read",
    "Group.Read.All",
    "Mail.ReadWrite",
    "Mail.Send",
    "Calendars.ReadWrite",
    "Contacts.ReadWrite",
    "Files.ReadWrite.All",
    "Sites.ReadWrite.All",
    "Team.ReadBasic.All",
    "Channel.ReadBasic.All",
    "TeamMember.Read.All",
    "ChannelMessage.Read.All",
    "Chat.ReadWrite",
    "Tasks.ReadWrite"
)

$AdminScopes = @(
    "Application.ReadWrite.All",
    "AppRoleAssignment.ReadWrite.All",
    "DelegatedPermissionGrant.ReadWrite.All",
    "Directory.Read.All",
    "User.Read.All",
    "Group.ReadWrite.All",
    "Channel.Create",
    "Sites.FullControl.All",
    "Tasks.ReadWrite"
)

function Import-GraphModules {
    $RequiredModules = @(
        "Microsoft.Graph.Authentication",
        "Microsoft.Graph.Applications",
        "Microsoft.Graph.Groups",
        "Microsoft.Graph.Identity.SignIns",
        "Microsoft.Graph.Teams",
        "Microsoft.Graph.Users"
    )

    foreach ($Module in $RequiredModules) {
        if (-not (Get-Module -ListAvailable -Name $Module)) {
            if (-not $InstallModules) {
                throw "Missing $Module. Re-run with -InstallModules or run: Install-Module $Module -Scope CurrentUser"
            }
            Write-Information "Installing missing PowerShell module $Module..." -InformationAction Continue
            Install-Module $Module -Scope CurrentUser -Repository PSGallery
        }
        Import-Module $Module
    }
}

function Connect-GraphAdministrator {
    Write-Information "Authenticating with Microsoft Graph. Follow the displayed device-login URL and code if prompted." -InformationAction Continue
    Connect-MgGraph `
        -Scopes $AdminScopes `
        -ContextScope CurrentUser `
        -UseDeviceCode `
        -ClientTimeout 600 `
        -NoWelcome
}

function Get-OneByDisplayName {
    param(
        [Parameter(Mandatory)] [AllowEmptyCollection()] [object[]] $Objects,
        [Parameter(Mandatory)] [string] $DisplayName,
        [Parameter(Mandatory)] [string] $Kind
    )

    $NamedObjects = @($Objects | Where-Object DisplayName -eq $DisplayName)
    if ($NamedObjects.Count -gt 1) {
        throw "More than one $Kind is named '$DisplayName'. Remove or rename duplicates before continuing."
    }
    return $NamedObjects | Select-Object -First 1
}

function Get-GraphServicePrincipal {
    return Get-MgServicePrincipal `
        -Filter "appId eq '$GraphAppId'" `
        -Property "id,appId,appRoles,oauth2PermissionScopes"
}

function Resolve-GraphAccess {
    param(
        [Parameter(Mandatory)] [object] $GraphServicePrincipal,
        [Parameter(Mandatory)] [string[]] $Names,
        [Parameter(Mandatory)] [ValidateSet("Role", "Scope")] [string] $Type
    )

    foreach ($Name in $Names) {
        $Permission = if ($Type -eq "Role") {
            $GraphServicePrincipal.AppRoles |
                Where-Object { $_.Value -eq $Name -and $_.IsEnabled }
        } else {
            $GraphServicePrincipal.Oauth2PermissionScopes |
                Where-Object { $_.Value -eq $Name -and $_.IsEnabled }
        }

        if (-not $Permission) {
            throw "Microsoft Graph $Type permission not found: $Name"
        }

        @{
            id = $Permission.Id
            type = $Type
        }
    }
}

function Get-OrCreateApplication {
    [CmdletBinding(SupportsShouldProcess = $true)]
    param(
        [Parameter(Mandatory)] [string] $DisplayName,
        [Parameter(Mandatory)] [object[]] $ResourceAccess,
        [switch] $PublicClient
    )

    $EscapedName = $DisplayName.Replace("'", "''")
    $Application = Get-OneByDisplayName `
        -Objects @(Get-MgApplication -Filter "displayName eq '$EscapedName'" -All) `
        -DisplayName $DisplayName `
        -Kind "application registration"

    $Body = @{
        displayName = $DisplayName
        signInAudience = "AzureADMyOrg"
        requiredResourceAccess = @(
            @{
                resourceAppId = $GraphAppId
                resourceAccess = $ResourceAccess
            }
        )
    }
    if ($PublicClient) {
        $Body.isFallbackPublicClient = $true
        $Body.publicClient = @{ redirectUris = @("http://localhost") }
    }

    if (-not $Application) {
        if (-not $PSCmdlet.ShouldProcess($DisplayName, "create application registration")) {
            return $null
        }
        $Application = New-MgApplication -BodyParameter $Body
    } else {
        Update-MgApplication -ApplicationId $Application.Id -BodyParameter $Body
        $Application = Get-MgApplication -ApplicationId $Application.Id
    }

    $ServicePrincipals = @(Get-MgServicePrincipal -Filter "appId eq '$($Application.AppId)'" -All)
    if ($ServicePrincipals.Count -gt 1) {
        throw "More than one service principal exists for application $($Application.AppId)."
    }
    $ServicePrincipal = $ServicePrincipals | Select-Object -First 1
    if (-not $ServicePrincipal) {
        $ServicePrincipal = New-MgServicePrincipal -BodyParameter @{ appId = $Application.AppId }
        Start-Sleep -Seconds 5
    }

    return [pscustomobject]@{
        Application = $Application
        ServicePrincipal = $ServicePrincipal
    }
}

function Grant-ApplicationPermissions {
    param(
        [Parameter(Mandatory)] [object] $GraphServicePrincipal,
        [Parameter(Mandatory)] [object] $ClientServicePrincipal,
        [Parameter(Mandatory)] [string[]] $PermissionNames
    )

    $Assignments = @(Get-MgServicePrincipalAppRoleAssignment `
        -ServicePrincipalId $GraphServicePrincipal.Id `
        -All)

    foreach ($PermissionName in $PermissionNames) {
        $Role = $GraphServicePrincipal.AppRoles |
            Where-Object { $_.Value -eq $PermissionName -and $_.IsEnabled }
        $Existing = $Assignments | Where-Object {
            $_.PrincipalId -eq $ClientServicePrincipal.Id -and $_.AppRoleId -eq $Role.Id
        }
        if (-not $Existing) {
            New-MgServicePrincipalAppRoleAssignment `
                -ServicePrincipalId $GraphServicePrincipal.Id `
                -BodyParameter @{
                    principalId = $ClientServicePrincipal.Id
                    resourceId = $GraphServicePrincipal.Id
                    appRoleId = $Role.Id
                } | Out-Null
        }
    }
}

function Grant-DelegatedPermissions {
    param(
        [Parameter(Mandatory)] [object] $GraphServicePrincipal,
        [Parameter(Mandatory)] [object] $ClientServicePrincipal,
        [Parameter(Mandatory)] [string[]] $PermissionNames
    )

    $ExpectedScope = $PermissionNames -join " "
    $ExistingGrants = @(Get-MgOauth2PermissionGrant -All |
        Where-Object {
            $_.ClientId -eq $ClientServicePrincipal.Id -and
            $_.ResourceId -eq $GraphServicePrincipal.Id -and
            $_.ConsentType -eq "AllPrincipals"
        })
    if ($ExistingGrants.Count -gt 1) {
        throw "More than one tenant-wide delegated permission grant exists for application $($ClientServicePrincipal.AppId)."
    }
    $Existing = $ExistingGrants | Select-Object -First 1

    if ($Existing) {
        Update-MgOauth2PermissionGrant `
            -OAuth2PermissionGrantId $Existing.Id `
            -BodyParameter @{ scope = $ExpectedScope } | Out-Null
        return
    }

    New-MgOauth2PermissionGrant -BodyParameter @{
        clientId = $ClientServicePrincipal.Id
        consentType = "AllPrincipals"
        resourceId = $GraphServicePrincipal.Id
        scope = $ExpectedScope
    } | Out-Null
}

function Test-LocalSecret {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo is required to inspect the encrypted AAI secret store."
    }

    $InventoryText = cargo run --quiet -- `
        --secrets-file $SecretsFile `
        --key-file $KeyFile `
        secrets list | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to inspect the encrypted AAI secret store."
    }

    $Inventory = $InventoryText | ConvertFrom-Json
    return @($Inventory.keys) -contains $AppSecretKey
}

function Initialize-AppSecret {
    param(
        [Parameter(Mandatory)] [string] $ApplicationObjectId
    )

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo is required to write the app secret to the encrypted AAI secret store."
    }

    $Application = Get-MgApplication -ApplicationId $ApplicationObjectId -Property "id,passwordCredentials"
    $OldCredentials = @($Application.PasswordCredentials | Where-Object DisplayName -eq "AAI CLI local E2E")
    $HasLocalSecret = Test-LocalSecret
    $CredentialIsUsable = $OldCredentials.Count -eq 1 -and
        $OldCredentials[0].EndDateTime -gt (Get-Date).AddDays(7)

    if ($CredentialIsUsable -and $HasLocalSecret) {
        Write-Information "Reusing the existing script-owned app credential and encrypted local secret." -InformationAction Continue
        return
    }

    $Credential = Add-MgApplicationPassword `
        -ApplicationId $ApplicationObjectId `
        -PasswordCredential @{
            displayName = "AAI CLI local E2E"
            endDateTime = (Get-Date).AddMonths(6)
        }

    $SecretsDirectory = Split-Path -Parent $SecretsFile
    $KeyDirectory = Split-Path -Parent $KeyFile
    if ($SecretsDirectory) { New-Item -ItemType Directory -Force $SecretsDirectory | Out-Null }
    if ($KeyDirectory) { New-Item -ItemType Directory -Force $KeyDirectory | Out-Null }

    try {
        $Credential.SecretText |
            cargo run --quiet -- `
                --secrets-file $SecretsFile `
                --key-file $KeyFile `
                secrets set $AppSecretKey | Out-Host
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to write $AppSecretKey to the encrypted AAI secret store."
        }
    } catch {
        Remove-MgApplicationPassword `
            -ApplicationId $ApplicationObjectId `
            -KeyId $Credential.KeyId `
            -ErrorAction SilentlyContinue
        throw
    }

    foreach ($OldCredential in $OldCredentials) {
        Remove-MgApplicationPassword -ApplicationId $ApplicationObjectId -KeyId $OldCredential.KeyId
    }
}

function Add-GroupDirectoryObject {
    param(
        [Parameter(Mandatory)] [string] $GroupId,
        [Parameter(Mandatory)] [ValidateSet("owners", "members")] [string] $Relationship,
        [Parameter(Mandatory)] [string] $ObjectId
    )

    $Existing = Invoke-MgGraphRequest `
        -Method GET `
        -Uri "/v1.0/groups/$GroupId/$Relationship?`$select=id"
    if ($Existing.value.id -contains $ObjectId) {
        return
    }

    Invoke-MgGraphRequest `
        -Method POST `
        -Uri "/v1.0/groups/$GroupId/$Relationship/`$ref" `
        -Body @{ "@odata.id" = "https://graph.microsoft.com/v1.0/directoryObjects/$ObjectId" } |
        Out-Null
}

function Get-OrCreateTeamGroup {
    param(
        [Parameter(Mandatory)] [object] $AdminUser,
        [Parameter(Mandatory)] [object] $TestUser
    )

    $EscapedName = $ResourceName.Replace("'", "''")
    $Group = Get-OneByDisplayName `
        -Objects @(Get-MgGroup -Filter "displayName eq '$EscapedName'" -All) `
        -DisplayName $ResourceName `
        -Kind "Microsoft 365 group"

    if (-not $Group) {
        $Suffix = Get-Date -Format "yyyyMMddHHmmss"
        $Group = New-MgGroup -BodyParameter @{
            displayName = $ResourceName
            description = "Disposable workspace for AAI CLI live tests"
            groupTypes = @("Unified")
            mailEnabled = $true
            mailNickname = "aai-cli-e2e-$Suffix"
            securityEnabled = $false
            visibility = "Private"
            "owners@odata.bind" = @(
                "https://graph.microsoft.com/v1.0/users/$($AdminUser.Id)",
                "https://graph.microsoft.com/v1.0/users/$($TestUser.Id)"
            ) | Select-Object -Unique
            "members@odata.bind" = @(
                "https://graph.microsoft.com/v1.0/users/$($AdminUser.Id)",
                "https://graph.microsoft.com/v1.0/users/$($TestUser.Id)"
            ) | Select-Object -Unique
        }
    } else {
        if (@($Group.GroupTypes) -notcontains "Unified") {
            throw "Existing group '$ResourceName' is not a Microsoft 365 group."
        }
        Add-GroupDirectoryObject -GroupId $Group.Id -Relationship owners -ObjectId $AdminUser.Id
        Add-GroupDirectoryObject -GroupId $Group.Id -Relationship owners -ObjectId $TestUser.Id
        Add-GroupDirectoryObject -GroupId $Group.Id -Relationship members -ObjectId $AdminUser.Id
        Add-GroupDirectoryObject -GroupId $Group.Id -Relationship members -ObjectId $TestUser.Id
    }

    return $Group
}

function Wait-ForTeam {
    param([Parameter(Mandatory)] [string] $GroupId)

    for ($Attempt = 1; $Attempt -le 30; $Attempt++) {
        try {
            return Invoke-MgGraphRequest -Method GET -Uri "/v1.0/teams/$GroupId"
        } catch {
            try {
                Set-MgGroupTeam -GroupId $GroupId -BodyParameter @{} | Out-Null
            } catch {
                Write-Verbose "Team provisioning attempt $Attempt is not ready: $($_.Exception.Message)"
            }
            Start-Sleep -Seconds 30
        }
    }
    throw "Team provisioning did not finish within 15 minutes. Re-run in discover or provision mode."
}

function Get-OrCreateChannel {
    param([Parameter(Mandatory)] [string] $TeamId)

    $Channels = @(Get-MgTeamChannel -TeamId $TeamId -All)
    $Channel = Get-OneByDisplayName -Objects $Channels -DisplayName $ChannelName -Kind "Teams channel"
    if (-not $Channel) {
        $Channel = New-MgTeamChannel -TeamId $TeamId -BodyParameter @{
            displayName = $ChannelName
            description = "AAI CLI behavioral E2E tests"
            membershipType = "standard"
        }
    }
    return $Channel
}

function Wait-ForSite {
    param([Parameter(Mandatory)] [string] $GroupId)

    for ($Attempt = 1; $Attempt -le 30; $Attempt++) {
        try {
            $Site = Invoke-MgGraphRequest `
                -Method GET `
                -Uri "/v1.0/groups/$GroupId/sites/root?`$select=id,webUrl,displayName"
            if ($Site.id) { return $Site }
        } catch {
            Write-Verbose "SharePoint provisioning attempt $Attempt is not ready: $($_.Exception.Message)"
        }
        Start-Sleep -Seconds 30
    }
    throw "SharePoint site provisioning did not finish within 15 minutes. Re-run in discover or provision mode."
}

function Get-DocumentDrive {
    param([Parameter(Mandatory)] [string] $SiteId)

    $Response = Invoke-MgGraphRequest `
        -Method GET `
        -Uri "/v1.0/sites/$SiteId/drives?`$select=id,name,driveType,webUrl"
    $Drive = $Response.value | Where-Object name -eq "Documents" | Select-Object -First 1
    if (-not $Drive) { $Drive = $Response.value | Select-Object -First 1 }
    if (-not $Drive) { throw "No document library was found for site $SiteId." }
    return $Drive
}

function Get-OrCreateList {
    param([Parameter(Mandatory)] [string] $SiteId)

    $Response = Invoke-MgGraphRequest `
        -Method GET `
        -Uri "/v1.0/sites/$SiteId/lists?`$select=id,displayName,webUrl"
    $List = Get-OneByDisplayName -Objects @($Response.value) -DisplayName $ListName -Kind "SharePoint list"
    if (-not $List) {
        $List = Invoke-MgGraphRequest -Method POST -Uri "/v1.0/sites/$SiteId/lists" -Body @{
            displayName = $ListName
            columns = @(
                @{
                    name = "Status"
                    choice = @{
                        choices = @("New", "InProgress", "Done")
                        displayAs = "dropDownMenu"
                        allowTextEntry = $false
                    }
                },
                @{ name = "ExternalId"; text = @{} }
            )
            list = @{ template = "genericList" }
        }
    }
    return $List
}

function Get-OrCreatePlan {
    param([Parameter(Mandatory)] [string] $GroupId)

    $Plans = Invoke-MgGraphRequest -Method GET -Uri "/v1.0/groups/$GroupId/planner/plans"
    $MatchingPlans = @($Plans.value | Where-Object title -eq $PlanName)
    if ($MatchingPlans.Count -gt 1) {
        throw "More than one Planner plan is named '$PlanName'. Remove or rename duplicates before continuing."
    }
    $Plan = $MatchingPlans | Select-Object -First 1
    if (-not $Plan) {
        $Plan = Invoke-MgGraphRequest -Method POST -Uri "/v1.0/planner/plans" -Body @{
            container = @{ url = "https://graph.microsoft.com/v1.0/groups/$GroupId" }
            title = $PlanName
        }
    }

    $Buckets = Invoke-MgGraphRequest -Method GET -Uri "/v1.0/planner/plans/$($Plan.id)/buckets"
    $MatchingBuckets = @($Buckets.value | Where-Object name -eq $BucketName)
    if ($MatchingBuckets.Count -gt 1) {
        throw "More than one Planner bucket is named '$BucketName'. Remove or rename duplicates before continuing."
    }
    $Bucket = $MatchingBuckets | Select-Object -First 1
    if (-not $Bucket) {
        $Bucket = Invoke-MgGraphRequest -Method POST -Uri "/v1.0/planner/buckets" -Body @{
            name = $BucketName
            planId = $Plan.id
            orderHint = " !"
        }
    }

    return [pscustomobject]@{ Plan = $Plan; Bucket = $Bucket }
}

function Write-EnvironmentFile {
    param(
        [Parameter(Mandatory)] [object] $Context,
        [Parameter(Mandatory)] [object] $TestUser,
        [Parameter(Mandatory)] [object] $AppOnlyApplication,
        [Parameter(Mandatory)] [object] $DelegatedApplication,
        [Parameter(Mandatory)] [object] $Site,
        [Parameter(Mandatory)] [object] $Drive,
        [Parameter(Mandatory)] [object] $List,
        [Parameter(Mandatory)] [string] $TeamId,
        [Parameter(Mandatory)] [object] $Channel,
        [Parameter(Mandatory)] [object] $Plan,
        [Parameter(Mandatory)] [object] $Bucket
    )

    $Environment = @"
AAI_E2E_MS_APP_PROFILE=microsoft-e2e-app
AAI_E2E_MS_DELEGATED_PROFILE=microsoft-e2e-delegated
AAI_E2E_MS_USER_ID=$($TestUser.Id)
AAI_E2E_MS_USER_UPN=$($TestUser.UserPrincipalName)
AAI_E2E_MS_SITE_ID=$($Site.id)
AAI_E2E_MS_SITE_URL=$($Site.webUrl)
AAI_E2E_MS_DRIVE_ID=$($Drive.id)
AAI_E2E_MS_LIST_ID=$($List.id)
AAI_E2E_MS_TEAM_ID=$TeamId
AAI_E2E_MS_CHANNEL_ID=$($Channel.Id)
AAI_E2E_MS_PLANNER_PLAN_ID=$($Plan.id)
AAI_E2E_MS_PLANNER_BUCKET_ID=$($Bucket.id)
AAI_E2E_MS_TENANT_ID=$($Context.TenantId)
AAI_E2E_MS_APP_CLIENT_ID=$($AppOnlyApplication.AppId)
AAI_E2E_MS_DELEGATED_CLIENT_ID=$($DelegatedApplication.AppId)
"@

    $Directory = Split-Path -Parent $EnvironmentPath
    if ($Directory) { New-Item -ItemType Directory -Force $Directory | Out-Null }
    Set-Content -Path $EnvironmentPath -Value $Environment -Encoding utf8
    Write-Information "Wrote non-secret E2E values to $EnvironmentPath" -InformationAction Continue
}

function Write-MicrosoftProfiles {
    param(
        [Parameter(Mandatory)] [object] $Context,
        [Parameter(Mandatory)] [object] $TestUser,
        [Parameter(Mandatory)] [object] $AppOnlyApplication,
        [Parameter(Mandatory)] [object] $DelegatedApplication
    )

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo is required to write Microsoft E2E profiles."
    }

    $AppProfile = @{
        provider = "microsoft"
        auth_type = "microsoft_client_credentials"
        base_url = "https://graph.microsoft.com/v1.0"
        tenant_id = $Context.TenantId
        client_id = $AppOnlyApplication.AppId
        user_id = $TestUser.Id
        client_secret_secret = $AppSecretKey
    } | ConvertTo-Json -Compress
    $DelegatedProfile = @{
        provider = "microsoft"
        auth_type = "microsoft_delegated"
        base_url = "https://graph.microsoft.com/v1.0"
        tenant_id = $Context.TenantId
        client_id = $DelegatedApplication.AppId
        user_id = $TestUser.Id
        refresh_token_secret = $DelegatedRefreshTokenKey
        scope = (@("offline_access", "openid", "profile") + $DelegatedPermissions | Select-Object -Unique) -join " "
    } | ConvertTo-Json -Compress

    cargo run --quiet -- --config $ConfigPath config profiles set microsoft-e2e-app --json $AppProfile | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Failed to write microsoft-e2e-app to $ConfigPath." }
    cargo run --quiet -- --config $ConfigPath config profiles set microsoft-e2e-delegated --json $DelegatedProfile | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Failed to write microsoft-e2e-delegated to $ConfigPath." }
}

function Remove-E2EResources {
    [CmdletBinding(SupportsShouldProcess = $true)]
    param()

    $EscapedName = $ResourceName.Replace("'", "''")
    $Groups = @(Get-MgGroup -Filter "displayName eq '$EscapedName'" -All)
    foreach ($Group in $Groups) {
        if ($PSCmdlet.ShouldProcess($Group.Id, "remove Team-backed Microsoft 365 group and its E2E resources")) {
            Remove-MgGroup -GroupId $Group.Id
        }
    }

    if ($RemoveApplications) {
        foreach ($DisplayName in @($AppOnlyName, $DelegatedName)) {
            $EscapedAppName = $DisplayName.Replace("'", "''")
            foreach ($Application in @(Get-MgApplication -Filter "displayName eq '$EscapedAppName'" -All)) {
                if ($PSCmdlet.ShouldProcess($Application.Id, "remove application registration $DisplayName")) {
                    Remove-MgApplication -ApplicationId $Application.Id
                }
            }
        }
    }
}

Import-GraphModules
Connect-GraphAdministrator
$Context = Get-MgContext
if (-not $Context.TenantId -or -not $Context.Account) {
    throw "Microsoft Graph did not return a tenant and signed-in account."
}

if ($Mode -eq "remove") {
    Remove-E2EResources
    Disconnect-MgGraph | Out-Null
    return
}

$TestUser = Get-MgUser -UserId $TestUserUpn -Property "id,displayName,userPrincipalName"
$AdminUser = Get-MgUser -UserId $Context.Account -Property "id,displayName,userPrincipalName"
$GraphServicePrincipal = Get-GraphServicePrincipal

$AppOnly = Get-OrCreateApplication `
    -DisplayName $AppOnlyName `
    -ResourceAccess @(Resolve-GraphAccess `
        -GraphServicePrincipal $GraphServicePrincipal `
        -Names $ApplicationPermissions `
        -Type Role)
$Delegated = Get-OrCreateApplication `
    -DisplayName $DelegatedName `
    -ResourceAccess @(Resolve-GraphAccess `
        -GraphServicePrincipal $GraphServicePrincipal `
        -Names $DelegatedPermissions `
        -Type Scope) `
    -PublicClient

if (-not $AppOnly -or -not $Delegated) {
    throw "Application provisioning was skipped. Run without -WhatIf to continue."
}

if ($Mode -eq "provision") {
    Grant-ApplicationPermissions `
        -GraphServicePrincipal $GraphServicePrincipal `
        -ClientServicePrincipal $AppOnly.ServicePrincipal `
        -PermissionNames $ApplicationPermissions
    Grant-DelegatedPermissions `
        -GraphServicePrincipal $GraphServicePrincipal `
        -ClientServicePrincipal $Delegated.ServicePrincipal `
        -PermissionNames $DelegatedPermissions
    Initialize-AppSecret -ApplicationObjectId $AppOnly.Application.Id
}

$Group = Get-OrCreateTeamGroup -AdminUser $AdminUser -TestUser $TestUser
$null = Wait-ForTeam -GroupId $Group.Id
$Channel = Get-OrCreateChannel -TeamId $Group.Id
$Site = Wait-ForSite -GroupId $Group.Id
$Drive = Get-DocumentDrive -SiteId $Site.id
$List = Get-OrCreateList -SiteId $Site.id
$Planner = Get-OrCreatePlan -GroupId $Group.Id

Write-EnvironmentFile `
    -Context $Context `
    -TestUser $TestUser `
    -AppOnlyApplication $AppOnly.Application `
    -DelegatedApplication $Delegated.Application `
    -Site $Site `
    -Drive $Drive `
    -List $List `
    -TeamId $Group.Id `
    -Channel $Channel `
    -Plan $Planner.Plan `
    -Bucket $Planner.Bucket

Write-MicrosoftProfiles `
    -Context $Context `
    -TestUser $TestUser `
    -AppOnlyApplication $AppOnly.Application `
    -DelegatedApplication $Delegated.Application

[pscustomobject]@{
    tenantId = $Context.TenantId
    userId = $TestUser.Id
    siteId = $Site.id
    driveId = $Drive.id
    listId = $List.id
    teamId = $Group.Id
    channelId = $Channel.Id
    plannerPlanId = $Planner.Plan.id
    plannerBucketId = $Planner.Bucket.id
    environmentPath = $EnvironmentPath
    configPath = $ConfigPath
    secretKey = $AppSecretKey
    delegatedRefreshTokenKey = $DelegatedRefreshTokenKey
} | ConvertTo-Json -Depth 4

Disconnect-MgGraph | Out-Null
