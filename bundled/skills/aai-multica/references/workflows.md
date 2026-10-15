# Multica workflows

All examples assume a configured profile named `multica`. Replace IDs with values discovered from the API. The installed executable is `aai-cli`; installations may provide an `aai` alias.

## Inbox → progress → reply → acknowledge

```sh
aai-cli --profile multica multica inbox unread-summary
aai-cli --profile multica multica inbox unread-count
aai-cli --profile multica multica inbox list --limit 100
aai-cli --profile multica multica issues get --issue-id ISSUE_ID
aai-cli --profile multica multica issues timeline --issue-id ISSUE_ID
aai-cli --profile multica multica issues comments list --issue-id ISSUE_ID --recent 5
aai-cli --profile multica multica issues comments list --issue-id ISSUE_ID --thread COMMENT_ID --tail 20
aai-cli --profile multica multica issues task-runs --issue-id ISSUE_ID
aai-cli --profile multica multica tasks messages --task-id TASK_ID --since 42
aai-cli --profile multica multica issues comments create --issue-id ISSUE_ID --parent-id COMMENT_ID --content 'Reviewed the result; please cover the remaining edge case.'
aai-cli --profile multica multica inbox read --inbox-id INBOX_ITEM_ID
```

Task-message `since` is a sequence number. Comment `since` is an RFC3339 timestamp:

```sh
aai-cli --profile multica multica issues comments list --issue-id ISSUE_ID --since 2026-10-15T08:00:00Z
aai-cli --profile multica multica inbox archived page --unread-only --limit 100
```

The inbox read call is a separate decision after triage. A failed reply does not acknowledge the notification. `inbox archive` archives sibling notifications for the same issue too.

Comment combinations follow the server: `tail` requires `thread`; `thread` conflicts with `recent`; `before` and `before-id` form a pair and require `recent` or `thread` plus `tail`. `roots-only` excludes thread/recent/tail/cursors. `fold` excludes since/tail/roots-only. `summary` is an independent, lossy content projection. The provider remains responsible for validating these combinations.

## Create, update, and assign

```sh
aai-cli --profile multica multica workspaces list
aai-cli --profile multica multica workspaces members list --workspace-id WORKSPACE_UUID
aai-cli --profile multica multica projects list
aai-cli --profile multica multica issue-statuses list
aai-cli --profile multica multica agents list
aai-cli --profile multica multica squads list
aai-cli --profile multica multica issues create --title 'Investigate the flaky test' --description 'Reproduce and document the failure.' --assignee-type agent --assignee-id AGENT_ID --project-id PROJECT_ID
aai-cli --profile multica multica issues update --issue-id ISSUE_ID --status STATUS_KEY --assignee-type squad --assignee-id SQUAD_ID
aai-cli --profile multica multica issues update --issue-id ISSUE_ID --json clear-assignment.json
```

`clear-assignment.json` can contain `{"assignee_type":null,"assignee_id":null}` where supported by the server. Other fields are not changed by the CLI. Use `--expected-revision` on supported issue/comment updates for optimistic concurrency.

## Comments and threads

```sh
aai-cli --profile multica multica comments update --comment-id COMMENT_ID --content 'Updated findings' --expected-revision 2
aai-cli --profile multica multica comments resolve --comment-id COMMENT_ID
aai-cli --profile multica multica comments unresolve --comment-id COMMENT_ID
aai-cli --profile multica multica comments reactions add --comment-id COMMENT_ID --emoji '👍'
aai-cli --profile multica multica comments delete-keep-replies --comment-id COMMENT_ID
```

## Squads and agents

```sh
aai-cli --profile multica multica squads create --name 'Release squad' --leader-id AGENT_ID
aai-cli --profile multica multica squads members add --squad-id SQUAD_ID --member-type agent --member-id AGENT_ID --role member
aai-cli --profile multica multica squads members status --squad-id SQUAD_ID
aai-cli --profile multica multica squads update --squad-id SQUAD_ID --leader-id NEW_LEADER_ID
aai-cli --profile multica multica agents create --name 'Reviewer' --runtime-id RUNTIME_ID --instructions 'Review assigned changes and report actionable findings.'
aai-cli --profile multica multica agents update --agent-id AGENT_ID --instructions 'Include regression scenarios.' --max-concurrent-tasks 2
aai-cli --profile multica multica agents skills add --agent-id AGENT_ID --json skills.json
aai-cli --profile multica multica agents skills set-enabled --agent-id AGENT_ID --skill-id SKILL_ID --enabled false
aai-cli --profile multica multica agents env set --agent-id AGENT_ID --json local/agent-env.json
```

Rotate the squad leader before removing its current leader. Permission and visibility restrictions are enforced by Multica. Complex agent runtime, MCP, skill, and environment payloads are forwarded unchanged from JSON; consult the endpoint's upstream request type linked in the command reference.

## Scheduling and administration

```sh
aai-cli --profile multica multica runtimes list
aai-cli --profile multica multica runtimes update --runtime-id RUNTIME_ID --custom-name 'Build machine'
aai-cli --profile multica multica runtime-profiles list --workspace-id WORKSPACE_UUID
aai-cli --profile multica multica autopilots cron-preview --expr '0 9 * * 1-5' --tz Europe/Berlin
aai-cli --profile multica multica autopilots create --json autopilot.json
aai-cli --profile multica multica autopilots triggers create --autopilot-id AUTOPILOT_ID --json trigger.json
aai-cli --profile multica multica autopilots runs list --autopilot-id AUTOPILOT_ID
aai-cli --profile multica multica issues wakeups create --issue-id ISSUE_ID --json wakeup.json
```

Autopilot configuration and its triggers are separate resources. Editing a wakeup instruction requires the provider's revision and expected prior instruction as well as the new text; supply them in JSON. No command installs or upgrades a daemon, waits for execution completion, or starts a monitoring process.
