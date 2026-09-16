# Changelog

All notable user-visible changes to `aai-cli` are recorded here.

## Unreleased

### Added

- Added durable Microsoft Graph app-only and delegated authentication. Delegated device login stores an encrypted refresh token once and automatically persists rotated refresh tokens for later unattended runs.
- Added typed Microsoft CLI commands for Outlook mail, calendar and contacts; OneDrive files; explicit SharePoint document-library upload/download/delete and list commands; Teams reads; Microsoft To Do lists and tasks; and Planner tasks.
- Added idempotent Microsoft 365 E2E provisioning, a noninteractive saved-credential/resource checker, and three ignored Given/When/Then live behavioral tests with cleanup guards.
- Added the bundled `aai-microsoft` Agent Skill with Microsoft 365 service-selection, resource-model, cross-service workflow, and command guidance; Microsoft command/auth documentation; and an architecture decision record.

### Changed

- Microsoft Graph `value` collections now participate in the shared pagination metadata contract, including `@odata.nextLink` discovery and `--limit` aggregation.
