# aai-cli Microsoft Graph command reference

Pass the profile before the service name:

```text
aai-cli --profile PROFILE microsoft <resource> <action> ...
```

## Authentication

```text
microsoft auth login
microsoft auth status
```

`auth login` is only for a `microsoft_delegated` profile. It performs one device-code login and saves the refresh token encrypted. Normal commands, including `auth status`, are noninteractive. A `microsoft_client_credentials` profile uses its encrypted client secret on every run.

Microsoft To Do supports delegated auth only. Other typed commands work when the selected profile has the corresponding Graph application or delegated permissions.

## Typed commands

```text
microsoft files upload <FILE> <PATH> [--drive-id ID | --user-id ID] [--mime-type TYPE]
microsoft files download <PATH> [--drive-id ID | --user-id ID] --output PATH
microsoft files delete <PATH> [--drive-id ID | --user-id ID]

microsoft sharepoint files upload <FILE> <PATH> --drive-id ID [--mime-type TYPE]
microsoft sharepoint files download <PATH> --drive-id ID --output PATH
microsoft sharepoint files delete <PATH> --drive-id ID

microsoft mail messages list [--user-id ID] [--limit N]
microsoft mail messages get <ID> [--user-id ID]
microsoft mail messages create [--user-id ID] --json JSON_OR_PATH
microsoft mail messages update <ID> [--user-id ID] --json JSON_OR_PATH
microsoft mail messages delete <ID> [--user-id ID]
microsoft mail send [--user-id ID] --json JSON_OR_PATH

microsoft calendar events list [--user-id ID] [--limit N]
microsoft calendar events get <ID> [--user-id ID]
microsoft calendar events create [--user-id ID] --json JSON_OR_PATH
microsoft calendar events update <ID> [--user-id ID] --json JSON_OR_PATH
microsoft calendar events delete <ID> [--user-id ID]

microsoft contacts list [--user-id ID] [--limit N]
microsoft contacts get <ID> [--user-id ID]
microsoft contacts create [--user-id ID] --json JSON_OR_PATH
microsoft contacts update <ID> [--user-id ID] --json JSON_OR_PATH
microsoft contacts delete <ID> [--user-id ID]

microsoft sharepoint lists list <SITE_ID> [--limit N]
microsoft sharepoint lists get <SITE_ID> <LIST_ID>
microsoft sharepoint items list <SITE_ID> <LIST_ID> [--limit N]
microsoft sharepoint items get <SITE_ID> <LIST_ID> <ITEM_ID>
microsoft sharepoint items create <SITE_ID> <LIST_ID> --json JSON_OR_PATH
microsoft sharepoint items update <SITE_ID> <LIST_ID> <ITEM_ID> --json JSON_OR_PATH
microsoft sharepoint items delete <SITE_ID> <LIST_ID> <ITEM_ID>

microsoft teams get <TEAM_ID>
microsoft teams channels <TEAM_ID> [--limit N]
microsoft teams channel <TEAM_ID> <CHANNEL_ID>
microsoft teams members <TEAM_ID> [--limit N]
microsoft teams messages <TEAM_ID> <CHANNEL_ID> [--limit N]
microsoft teams chats [--user-id ID] [--limit N]

microsoft todo lists list [--user-id ID] [--limit N]
microsoft todo lists get <ID> [--user-id ID]
microsoft todo lists create [--user-id ID] --json JSON_OR_PATH
microsoft todo lists update <ID> [--user-id ID] --json JSON_OR_PATH
microsoft todo lists delete <ID> [--user-id ID]
microsoft todo tasks list <LIST_ID> [--user-id ID] [--limit N]
microsoft todo tasks get <LIST_ID> <TASK_ID> [--user-id ID]
microsoft todo tasks create <LIST_ID> [--user-id ID] --json JSON_OR_PATH
microsoft todo tasks update <LIST_ID> <TASK_ID> [--user-id ID] --json JSON_OR_PATH
microsoft todo tasks delete <LIST_ID> <TASK_ID> [--user-id ID]

microsoft planner plans get <ID>
microsoft planner buckets get <ID>
microsoft planner tasks get <ID>
microsoft planner tasks create --json JSON_OR_PATH
microsoft planner tasks update <ID> --etag ETAG --json JSON_OR_PATH
microsoft planner tasks delete <ID> --etag ETAG
```

All create/update bodies must be JSON objects. `--json` accepts inline JSON, a file path, or `-` for stdin. SharePoint item creation normally uses `{ "fields": { ... } }`; item update accepts the fields object itself. Planner create normally includes `planId`, `bucketId`, and `title`.

Planner uses optimistic concurrency. Read `@odata.etag` from the current task and pass it unchanged to `--etag`; a stale value fails instead of overwriting another writer.

## Generic Graph request

```text
microsoft request <get|head|post|put|patch|delete> <RELATIVE_PATH> \
  [--query KEY=VALUE] [--json JSON_OR_PATH] [--allow-write]
```

Use a Graph path such as `/me` or `/users/{id}/messages`, not an arbitrary host URL. Mutating methods require `--allow-write`.

## Full create/read/update/delete pattern

1. Create the resource with a distinctive, run-specific title and retain its returned ID.
2. Get it by ID and verify the important fields.
3. Update it and get it again. For Planner, carry forward the newest ETag.
4. Delete it.
5. Get it once more and require a structured `not_found` error when that endpoint provides immediate deletion visibility.
6. Keep a best-effort cleanup action registered from the moment creation succeeds, so partial failures remain recoverable.

For files, download and validate the bytes before deleting. File content is never printed to stdout.

## Output and errors

Provider objects are returned without translating their field names. Aggregated list responses retain the Graph `{ "value": [...] }` shape and gain `_aai.pagination`. Errors are JSON on stderr with `code`, `service`, `operation`, `status`, and provider `details`; any error exits nonzero.
