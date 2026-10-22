# ADR 0001: Microsoft Graph integration and durable agent credentials

- Status: Accepted
- Date: 2026-09-16

## Context

Agents need repeatable access to Microsoft 365 without a person signing in for every run. The initial coverage must span Outlook, OneDrive, SharePoint, Teams, Microsoft To Do, and Planner while preserving the CLI's JSON output, structured errors, provider response shapes, and centralized authentication behavior.

Microsoft Graph has two materially different authorization models. Client credentials are suitable for unattended organization-owned work, while delegated authorization preserves a user's identity and is the consistent model for complete Microsoft To Do CRUD across operations. Delegated access tokens are short-lived and Microsoft can rotate the refresh token when it is used.

## Decision

Expose one `microsoft` top-level command with typed resource groups and retain `microsoft request` as the escape hatch for unsupported Graph endpoints.

- Keep token acquisition in the shared OAuth/HTTP path. Support `microsoft_client_credentials` and `microsoft_delegated` profiles.
- Bootstrap delegated access once with device authorization, store the refresh token only in the encrypted secret store, and atomically replace it when Microsoft returns a rotated token.
- Keep Graph endpoint behavior in focused Microsoft service modules: Outlook, SharePoint, Teams, To Do, Planner, and files/auth dispatch.
- Preserve Graph JSON field names and `value` collections. Aggregate `@odata.nextLink` pages only up to the caller's `--limit`.
- Require Planner ETags for update/delete and reject Microsoft To Do app-only profiles before making a network request.
- Verify the system boundary with ignored, disposable live CLI tests and an idempotent provisioning/checking runbook.

## Alternatives considered

1. Depend on Microsoft Graph PowerShell at runtime. Rejected because it would add a large runtime dependency and reintroduce interactive session state. PowerShell remains provisioning-only.
2. Persist access tokens. Rejected because they expire quickly and do not provide durable unattended access.
3. Create separate top-level commands for every Microsoft product. Rejected because they share one Graph authentication and pagination model; a single namespace makes profile and generic-request behavior consistent.
4. Use app-only authorization exclusively. Rejected because application-permission support varies across Microsoft To Do operations, while complete list/task CRUD needs one consistent actor model.

## Consequences

- Agents can run repeatedly with saved encrypted credentials; a user is only needed for initial delegated consent or reauthorization after revocation.
- Tenant administrators must grant broad test permissions deliberately. Production deployments should provision least-privilege applications for their actual command subset.
- Rotated delegated refresh-token writes mean concurrent processes using the same profile should be avoided unless secret-store coordination is added later.
- The typed surface is additive. Existing profiles and commands need no migration. Removing the Microsoft profiles/secrets and reverting this change is a safe rollback; no application database or provider-data migration exists.
- Another in-flight SharePoint-only branch may need reconciliation before both can merge because it touches the same auth, CLI, pagination, and service registration boundaries.
