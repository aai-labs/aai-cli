#Requires -Version 7.2

[CmdletBinding()]
param(
    [string] $EnvironmentPath = "local/e2e-ms.env",
    [string] $ConfigPath = "local/e2e.config.toml"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RequiredEnvironmentKeys = @(
    "AAI_E2E_MS_APP_PROFILE",
    "AAI_E2E_MS_DELEGATED_PROFILE",
    "AAI_E2E_MS_USER_ID",
    "AAI_E2E_MS_SITE_ID",
    "AAI_E2E_MS_SITE_URL",
    "AAI_E2E_MS_DRIVE_ID",
    "AAI_E2E_MS_LIST_ID",
    "AAI_E2E_MS_TEAM_ID",
    "AAI_E2E_MS_CHANNEL_ID",
    "AAI_E2E_MS_PLANNER_PLAN_ID",
    "AAI_E2E_MS_PLANNER_BUCKET_ID"
)

function Read-EnvironmentFile {
    param([Parameter(Mandatory)] [string] $Path)

    if (-not (Test-Path $Path -PathType Leaf)) {
        throw "Microsoft E2E environment file not found: $Path. Run scripts/ms-e2e.ps1 first."
    }
    $Values = @{}
    foreach ($Line in Get-Content $Path) {
        if ($Line -match '^([^#][^=]+)=(.*)$') {
            $Values[$Matches[1].Trim()] = $Matches[2].Trim()
        }
    }
    foreach ($Key in $RequiredEnvironmentKeys) {
        if (-not $Values.ContainsKey($Key) -or [string]::IsNullOrWhiteSpace($Values[$Key])) {
            throw "Missing $Key in $Path. Re-run scripts/ms-e2e.ps1 in discover mode."
        }
    }
    return $Values
}

function Invoke-Aai {
    param(
        [Parameter(Mandatory)] [string] $Profile,
        [Parameter(Mandatory)] [string[]] $Arguments
    )

    $Output = & cargo run --quiet -- --config $ConfigPath --profile $Profile @Arguments | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw "aai-cli failed for profile '$Profile': $($Arguments -join ' ')"
    }
    return $Output | ConvertFrom-Json -Depth 100
}

function Assert-Equal {
    param(
        [Parameter(Mandatory)] [object] $Actual,
        [Parameter(Mandatory)] [object] $Expected,
        [Parameter(Mandatory)] [string] $Message
    )
    if ($Actual -ne $Expected) {
        throw "$Message Expected '$Expected', got '$Actual'."
    }
}

function Invoke-Check {
    param(
        [Parameter(Mandatory)] [string] $Resource,
        [Parameter(Mandatory)] [scriptblock] $Action
    )

    $Started = Get-Date
    try {
        & $Action
        $script:Checks.Add([pscustomobject]@{
            resource = $Resource
            status = "passed"
            durationMs = [math]::Round(((Get-Date) - $Started).TotalMilliseconds)
            error = $null
        })
    } catch {
        $script:Checks.Add([pscustomobject]@{
            resource = $Resource
            status = "failed"
            durationMs = [math]::Round(((Get-Date) - $Started).TotalMilliseconds)
            error = $_.Exception.Message
        })
    }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo is required to run the Microsoft E2E resource checker."
}
if (-not (Test-Path $ConfigPath -PathType Leaf)) {
    throw "Microsoft E2E config not found: $ConfigPath. Re-run scripts/ms-e2e.ps1."
}

$Environment = Read-EnvironmentFile -Path $EnvironmentPath
$Checks = [System.Collections.Generic.List[object]]::new()
$AppProfile = $Environment.AAI_E2E_MS_APP_PROFILE
$DelegatedProfile = $Environment.AAI_E2E_MS_DELEGATED_PROFILE

Invoke-Check -Resource "saved app credential" -Action {
    $Status = Invoke-Aai -Profile $AppProfile -Arguments @("microsoft", "auth", "status")
    Assert-Equal $Status.authenticated $true "App authentication failed."
    Assert-Equal $Status.mode "application" "Unexpected app authentication mode."
}

Invoke-Check -Resource "saved delegated credential" -Action {
    $Status = Invoke-Aai -Profile $DelegatedProfile -Arguments @("microsoft", "auth", "status")
    Assert-Equal $Status.authenticated $true "Delegated authentication failed."
    Assert-Equal $Status.identity.id $Environment.AAI_E2E_MS_USER_ID "Unexpected delegated user."
}

Invoke-Check -Resource "test user" -Action {
    $User = Invoke-Aai -Profile $AppProfile -Arguments @(
        "microsoft", "request", "get", "/users/$($Environment.AAI_E2E_MS_USER_ID)",
        "--query", "`$select=id,userPrincipalName,accountEnabled"
    )
    Assert-Equal $User.id $Environment.AAI_E2E_MS_USER_ID "Unexpected test user."
    Assert-Equal $User.accountEnabled $true "Test user is disabled."
}

Invoke-Check -Resource "Team group and test-user membership" -Action {
    $Group = Invoke-Aai -Profile $AppProfile -Arguments @(
        "microsoft", "request", "get", "/groups/$($Environment.AAI_E2E_MS_TEAM_ID)",
        "--query", "`$select=id,displayName,groupTypes"
    )
    Assert-Equal $Group.id $Environment.AAI_E2E_MS_TEAM_ID "Unexpected Team group."
    if (@($Group.groupTypes) -notcontains "Unified") { throw "Team group is not a Microsoft 365 Unified group." }

    $Member = Invoke-Aai -Profile $AppProfile -Arguments @(
        "microsoft", "request", "get",
        "/groups/$($Environment.AAI_E2E_MS_TEAM_ID)/members/$($Environment.AAI_E2E_MS_USER_ID)",
        "--query", "`$select=id"
    )
    Assert-Equal $Member.id $Environment.AAI_E2E_MS_USER_ID "Test user is not a direct Team member."
}

Invoke-Check -Resource "Team and channel" -Action {
    $Team = Invoke-Aai -Profile $AppProfile -Arguments @(
        "microsoft", "request", "get", "/teams/$($Environment.AAI_E2E_MS_TEAM_ID)",
        "--query", "`$select=id,displayName"
    )
    $Channel = Invoke-Aai -Profile $AppProfile -Arguments @(
        "microsoft", "request", "get",
        "/teams/$($Environment.AAI_E2E_MS_TEAM_ID)/channels/$($Environment.AAI_E2E_MS_CHANNEL_ID)",
        "--query", "`$select=id,displayName,membershipType"
    )
    Assert-Equal $Team.id $Environment.AAI_E2E_MS_TEAM_ID "Unexpected Team."
    Assert-Equal $Channel.id $Environment.AAI_E2E_MS_CHANNEL_ID "Unexpected channel."
}

Invoke-Check -Resource "SharePoint site, drive, and list" -Action {
    $Site = Invoke-Aai -Profile $AppProfile -Arguments @(
        "microsoft", "request", "get", "/sites/$($Environment.AAI_E2E_MS_SITE_ID)",
        "--query", "`$select=id,webUrl"
    )
    $Drive = Invoke-Aai -Profile $AppProfile -Arguments @(
        "microsoft", "request", "get", "/drives/$($Environment.AAI_E2E_MS_DRIVE_ID)",
        "--query", "`$select=id,driveType"
    )
    $List = Invoke-Aai -Profile $AppProfile -Arguments @(
        "microsoft", "request", "get",
        "/sites/$($Environment.AAI_E2E_MS_SITE_ID)/lists/$($Environment.AAI_E2E_MS_LIST_ID)",
        "--query", "`$select=id,displayName"
    )
    Assert-Equal $Site.id $Environment.AAI_E2E_MS_SITE_ID "Unexpected SharePoint site."
    Assert-Equal $Site.webUrl $Environment.AAI_E2E_MS_SITE_URL "Unexpected SharePoint URL."
    Assert-Equal $Drive.id $Environment.AAI_E2E_MS_DRIVE_ID "Unexpected document drive."
    Assert-Equal $Drive.driveType "documentLibrary" "Configured drive is not a document library."
    Assert-Equal $List.id $Environment.AAI_E2E_MS_LIST_ID "Unexpected SharePoint list."
}

Invoke-Check -Resource "Planner plan and bucket" -Action {
    $Plan = Invoke-Aai -Profile $DelegatedProfile -Arguments @(
        "microsoft", "request", "get", "/planner/plans/$($Environment.AAI_E2E_MS_PLANNER_PLAN_ID)"
    )
    $Bucket = Invoke-Aai -Profile $DelegatedProfile -Arguments @(
        "microsoft", "request", "get", "/planner/buckets/$($Environment.AAI_E2E_MS_PLANNER_BUCKET_ID)"
    )
    Assert-Equal $Plan.id $Environment.AAI_E2E_MS_PLANNER_PLAN_ID "Unexpected Planner plan."
    Assert-Equal $Plan.container.containerId $Environment.AAI_E2E_MS_TEAM_ID "Planner plan is not attached to the Team group."
    Assert-Equal $Bucket.id $Environment.AAI_E2E_MS_PLANNER_BUCKET_ID "Unexpected Planner bucket."
    Assert-Equal $Bucket.planId $Environment.AAI_E2E_MS_PLANNER_PLAN_ID "Planner bucket is attached to another plan."
}

$Failed = @($Checks | Where-Object status -eq "failed")
[pscustomobject]@{
    status = if ($Failed.Count -eq 0) { "passed" } else { "failed" }
    checks = $Checks
} | ConvertTo-Json -Depth 10

if ($Failed.Count -gt 0) { exit 1 }
