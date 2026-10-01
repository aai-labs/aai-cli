# Changelog

All notable user-visible changes to `aai-cli` are recorded here.

## Unreleased

### Added

- Added `pipedrive files list`, `files get`, and `files download --output` for file attachments on deals, persons, and organizations, so meeting transcripts and other documents can be retrieved.
- Added `pipedrive fields deals|persons|organizations|activities list/get`, returning field definitions with option labels so custom-field keys and option IDs in record payloads can be resolved to names. `list` defaults to `--limit 500`.
- Added `pipedrive users list/get/me/find`, `pipelines list/get`, and `stages list/get` for resolving owner, pipeline, and stage IDs.
- Added `pipedrive notes create/update/delete` and `activities create/update/delete`.
- Added `pipedrive persons|organizations|deals merge <id> --merge-with-id ID`.
- Added `--add-label-ids` and `--remove-label-ids` to Pipedrive lead, person, organization, and deal updates; they change individual labels while keeping the record's others, unlike `--label-ids`, which replaces the set.
- Added durable Microsoft Graph app-only and delegated authentication. Delegated device login stores an encrypted refresh token once and automatically persists rotated refresh tokens for later unattended runs.
- Added typed Microsoft CLI commands for Outlook mail, calendar and contacts; OneDrive files; explicit SharePoint document-library upload/download/delete and list commands; Teams reads; Microsoft To Do lists and tasks; and Planner tasks.
- Added idempotent Microsoft 365 E2E provisioning, a noninteractive saved-credential/resource checker, and ignored Given/When/Then live behavioral tests with cleanup guards.
- Added the bundled `aai-microsoft` Agent Skill with Microsoft 365 service-selection, resource-model, cross-service workflow, and command guidance; Microsoft command/auth documentation; and an architecture decision record.
- Added delegated Microsoft Graph Excel worksheet, range, table, and table-row commands for OneDrive and SharePoint workbooks. Word remains file-transfer-only: download, edit externally, and upload the complete replacement.

### Changed

- Pipedrive list aggregation requests only the records still needed, so a limit above 500 that is not a multiple of 500 no longer leaves the tail of the last page behind the continuation marker. Aggregates now report their final state: v2 lists end with `next_cursor: null` and v1 lists with the correct `more_items_in_collection`, and `_aai.pagination` treats Pipedrive's null cursor as `complete` instead of `unknown`.
- File downloads now follow redirects themselves and send credentials only to the original origin. Previously a provider-specific auth header, such as Pipedrive's `x-api-token`, was forwarded to a cross-origin redirect target. The origin includes the scheme, so an http to https hop on the same host also drops credentials, where reqwest compared only host and port.
- Pipedrive `--add-label-ids` / `--remove-label-ids` refuse to write when the record comes back without `label_ids` instead of treating it as unlabelled, which would have erased its labels; a literal `null` still means no labels. `pipedrive users list` now reports `_aai.pagination.status: complete`, since Pipedrive returns every user in one response.
- Microsoft Graph `value` collections now participate in the shared pagination metadata contract, including `@odata.nextLink` discovery and `--limit` aggregation.
