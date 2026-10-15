# Multica command reference

Contract: application API at revision `83527872634824012dd26bb183aa216fab631c05`. All commands start with `aai-cli --profile multica multica`. Use `--help` on any resource/action.

Path placeholders are required flags: `{issue_id}` becomes `--issue-id`. Query/body field names become hyphenated flags. Boolean flags accept `--enabled`, `--enabled true`, or `--enabled false`. Writes also accept `--json FILE|-` (and inline JSON); typed fields override JSON. Complex fields not listed in the table must be supplied through JSON. GET commands accept repeated `--query KEY=VALUE` for additional upstream parameters; workspace and paging parameters use their dedicated flags.

All collection commands have `--limit` (total cap, default 100), and `--page-skip` for emitted continuation commands. Offset endpoints additionally accept `--offset`. Empty `—` cells mean no additional typed fields. For JSON-only operations, follow the linked upstream handler/request type.

Pagination modes: **offset** and **offset-until-short** aggregate server pages; **cursor** uses `next_cursor`; **comments** uses header cursor pairs; **agent-tasks** uses a header cursor; **single** fetches the provider view once and applies the total cap; **capped** cannot prove full history. **none** means a singleton/object response.

## Endpoint map

### Issues

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `issues list` | `GET /issues` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue.go)) | `--q`, `--status`, `--statuses`, `--status-category`, `--status-categories`, `--priority`, `--priorities`, `--assignee-id`, `--assignee-ids`, `--assignee-types`, `--assignee-filters`, `--creator-id`, `--creator-filters`, `--project-id`, `--project-ids`, `--label-ids`, `--sort`, `--direction`, `--metadata`, `--properties`, `--scheduled`, `--top-level-only`, `--include-no-assignee`, `--include-no-project`, `--involves-user-id`, `--ids`, `--open-only` | — | offset |
| `issues search` | `GET /issues/search` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue.go)) | `--q`, `--include-closed` | — | offset |
| `issues get` | `GET /issues/{issue_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue.go)) | — | — | none |
| `issues create` | `POST /issues` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue.go)) | — | `--title`, `--description`, `--status`, `--priority`, `--assignee-type`, `--assignee-id`, `--project-id`, `--parent-issue-id`, `--start-date`, `--due-date` | none |
| `issues update` | `PUT /issues/{issue_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue.go)) | — | `--title`, `--description`, `--status`, `--priority`, `--assignee-type`, `--assignee-id`, `--project-id`, `--parent-issue-id`, `--start-date`, `--due-date`, `--expected-revision` | none |
| `issues delete` | `DELETE /issues/{issue_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue.go)) | — | — | none |
| `issues move` | `POST /issues/{issue_id}/move` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_move.go)) | — | — | none |
| `issues rerun` | `POST /issues/{issue_id}/rerun` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/task_lifecycle.go)) | — | — | none |
| `issues children` | `GET /issues/{issue_id}/children` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue.go)) | — | — | single |
| `issues timeline` | `GET /issues/{issue_id}/timeline` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/activity.go)) | `--before`, `--after`, `--around` | — | single |
| `issues subscribers` | `GET /issues/{issue_id}/subscribers` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/subscriber.go)) | — | — | single |
| `issues labels list` | `GET /issues/{issue_id}/labels` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | — | single |
| `issues metadata list` | `GET /issues/{issue_id}/metadata` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_metadata.go)) | — | — | none |
| `issues pull-requests list` | `GET /issues/{issue_id}/pull-requests` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/github.go)) | — | — | single |
| `issues task-runs` | `GET /issues/{issue_id}/task-runs` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/daemon.go)) | `--scope`, `--active` | — | single |
| `issues subscribe` | `POST /issues/{issue_id}/subscribe` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/subscriber.go)) | — | — | none |
| `issues unsubscribe` | `POST /issues/{issue_id}/unsubscribe` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/subscriber.go)) | — | — | none |
| `issues unsubscribe-subtree` | `POST /issues/{issue_id}/unsubscribe/subtree` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/subscriber.go)) | — | — | none |
| `issues active-task` | `GET /issues/{issue_id}/active-task` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/daemon.go)) | — | — | none |
| `issues usage` | `GET /issues/{issue_id}/usage` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/daemon.go)) | — | — | none |
| `issues metadata set` | `PUT /issues/{issue_id}/metadata/{key}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_metadata.go)) | — | `--value` | none |
| `issues metadata delete` | `DELETE /issues/{issue_id}/metadata/{key}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_metadata.go)) | — | — | none |
| `issues properties set` | `PUT /issues/{issue_id}/properties/{property_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/property.go)) | — | — | none |
| `issues properties delete` | `DELETE /issues/{issue_id}/properties/{property_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/property.go)) | — | — | none |
| `issues pull-requests link` | `POST /issues/{issue_id}/pull-requests` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/pr_auto_complete.go)) | — | — | none |
| `issues pull-requests unlink` | `DELETE /issues/{issue_id}/pull-requests/{pr_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/pr_auto_complete.go)) | — | — | none |
| `issues pr-auto-complete` | `PUT /issues/{issue_id}/pr-auto-complete` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/pr_auto_complete.go)) | — | `--disabled` | none |
| `issues labels add` | `POST /issues/{issue_id}/labels` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | `--label-id` | none |
| `issues labels remove` | `DELETE /issues/{issue_id}/labels/{label_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | — | none |

### Comments

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `issues comments list` | `GET /issues/{issue_id}/comments` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/comment.go)) | `--since`, `--thread`, `--tail`, `--recent`, `--before`, `--before-id`, `--roots-only`, `--summary`, `--fold` | — | comments |
| `issues comments create` | `POST /issues/{issue_id}/comments` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/comment.go)) | — | `--content`, `--type`, `--parent-id` | none |
| `issues comments trigger-preview` | `POST /issues/{issue_id}/comments/trigger-preview` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/comment.go)) | — | `--content` | none |
| `comments update` | `PUT /comments/{comment_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/comment.go)) | — | `--content`, `--expected-revision` | none |
| `comments delete` | `DELETE /comments/{comment_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/comment.go)) | — | — | none |
| `comments delete-keep-replies` | `DELETE /comments/{comment_id}/keep-replies` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/comment.go)) | — | — | none |
| `comments resolve` | `POST /comments/{comment_id}/resolve` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/comment.go)) | — | — | none |
| `comments unresolve` | `DELETE /comments/{comment_id}/resolve` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/comment.go)) | — | — | none |
| `comments reactions add` | `POST /comments/{comment_id}/reactions` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/reaction.go)) | — | `--emoji` | none |
| `comments reactions remove` | `DELETE /comments/{comment_id}/reactions` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/reaction.go)) | — | `--emoji` | none |

### Inbox

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `inbox list` | `GET /inbox` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | single |
| `inbox archived list` | `GET /inbox/archived` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | capped |
| `inbox archived page` | `GET /inbox/archived/page` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox_archive.go)) | `--statuses`, `--priorities`, `--actors`, `--unread-only`, `--group-id`, `--cursor` | — | cursor |
| `inbox archived facets` | `GET /inbox/archived/facets` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox_archive.go)) | `--statuses`, `--priorities`, `--actors`, `--unread-only` | — | none |
| `inbox unread-count` | `GET /inbox/unread-count` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox unread-summary` | `GET /inbox/unread-summary` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox mark-all-read` | `POST /inbox/mark-all-read` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox archive-all` | `POST /inbox/archive-all` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox archive-all-read` | `POST /inbox/archive-all-read` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox archive-completed` | `POST /inbox/archive-completed` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox read` | `POST /inbox/{inbox_id}/read` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox unread` | `POST /inbox/{inbox_id}/unread` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox archive` | `POST /inbox/{inbox_id}/archive` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |
| `inbox unarchive` | `POST /inbox/{inbox_id}/unarchive` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/inbox.go)) | — | — | none |

### Squads

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `squads list` | `GET /squads` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | — | single |
| `squads get` | `GET /squads/{squad_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | — | none |
| `squads create` | `POST /squads` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | `--name`, `--description`, `--leader-id`, `--avatar-url` | none |
| `squads update` | `PUT /squads/{squad_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | `--name`, `--description`, `--leader-id`, `--avatar-url`, `--instructions` | none |
| `squads delete` | `DELETE /squads/{squad_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | — | none |
| `squads members list` | `GET /squads/{squad_id}/members` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | — | single |
| `squads members status` | `GET /squads/{squad_id}/members/status` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | — | single |
| `squads members add` | `POST /squads/{squad_id}/members` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | `--member-type`, `--member-id`, `--role` | none |
| `squads members remove` | `DELETE /squads/{squad_id}/members` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | `--member-type`, `--member-id` | none |
| `squads members update-role` | `PATCH /squads/{squad_id}/members/role` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/squad.go)) | — | `--member-type`, `--member-id`, `--role` | none |

### Agents

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `agents list` | `GET /agents` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent.go)) | `--include-archived` | — | single |
| `agents get` | `GET /agents/{agent_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent.go)) | — | — | none |
| `agents create` | `POST /agents` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent.go)) | — | `--name`, `--description`, `--instructions`, `--avatar-url`, `--runtime-id`, `--visibility`, `--permission-mode`, `--max-concurrent-tasks`, `--model`, `--thinking-level`, `--service-tier` | none |
| `agents update` | `PUT /agents/{agent_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent.go)) | — | `--name`, `--description`, `--instructions`, `--avatar-url`, `--runtime-id`, `--visibility`, `--permission-mode`, `--max-concurrent-tasks`, `--model`, `--thinking-level`, `--service-tier` | none |
| `agents archive` | `POST /agents/{agent_id}/archive` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent.go)) | — | — | none |
| `agents restore` | `POST /agents/{agent_id}/restore` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent.go)) | — | — | none |
| `agents cancel-tasks` | `POST /agents/{agent_id}/cancel-tasks` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent.go)) | — | — | none |
| `agents tasks` | `GET /agents/{agent_id}/tasks` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent.go)) | `--before`, `--include-usage` | — | agenttasks |
| `agents skills list` | `GET /agents/{agent_id}/skills` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | — | single |
| `agents skills add` | `POST /agents/{agent_id}/skills/add` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | — | none |
| `agents skills set` | `PUT /agents/{agent_id}/skills` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | — | none |
| `agents skills remove` | `DELETE /agents/{agent_id}/skills/{skill_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | — | none |
| `agents skills set-enabled` | `PUT /agents/{agent_id}/skills/{skill_id}/enabled` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | `--enabled` | none |
| `agents mcp-servers list` | `GET /agents/{agent_id}/mcp-servers` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace_mcp_api.go)) | — | — | single |
| `agents mcp-servers add` | `POST /agents/{agent_id}/mcp-servers` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace_mcp_api.go)) | — | — | none |
| `agents mcp-servers remove` | `DELETE /agents/{agent_id}/mcp-servers/{server_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace_mcp_api.go)) | — | — | none |
| `agents mcp-servers set-enabled` | `PUT /agents/{agent_id}/mcp-servers/{server_id}/enabled` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace_mcp_api.go)) | — | `--enabled` | none |
| `agents runtime-skills set-enabled` | `PUT /agents/{agent_id}/runtime-skills/enabled` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent_runtime_skills.go)) | — | `--enabled` | none |
| `agents env get` | `GET /agents/{agent_id}/env` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent_env.go)) | — | — | none |
| `agents env set` | `PUT /agents/{agent_id}/env` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/agent_env.go)) | — | — | none |

### Projects

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `projects list` | `GET /projects` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project.go)) | `--status`, `--priority` | — | single |
| `projects get` | `GET /projects/{project_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project.go)) | — | — | none |
| `projects create` | `POST /projects` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project.go)) | — | `--title`, `--description`, `--icon`, `--status`, `--priority`, `--lead-type`, `--lead-id`, `--start-date`, `--due-date` | none |
| `projects update` | `PUT /projects/{project_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project.go)) | — | `--title`, `--description`, `--icon`, `--status`, `--priority`, `--lead-type`, `--lead-id`, `--start-date`, `--due-date` | none |
| `projects delete` | `DELETE /projects/{project_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project.go)) | — | — | none |
| `projects search` | `GET /projects/search` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project.go)) | `--q`, `--include-closed` | — | offset |
| `projects resources list` | `GET /projects/{project_id}/resources` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project_resource.go)) | — | — | single |
| `projects resources create` | `POST /projects/{project_id}/resources` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project_resource.go)) | — | `--resource-type`, `--label`, `--position` | none |
| `projects resources update` | `PUT /projects/{project_id}/resources/{resource_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project_resource.go)) | — | `--label`, `--position` | none |
| `projects resources delete` | `DELETE /projects/{project_id}/resources/{resource_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/project_resource.go)) | — | — | none |

### Workspaces

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `workspaces list` | `GET /workspaces` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace.go)) | — | — | single |
| `workspaces get` | `GET /workspaces/{workspace_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace.go)) | — | — | none |
| `workspaces create` | `POST /workspaces` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace.go)) | — | `--name`, `--slug`, `--description` | none |
| `workspaces update` | `PATCH /workspaces/{workspace_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace.go)) | — | `--name`, `--description` | none |
| `workspaces members list` | `GET /workspaces/{workspace_id}/members` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace.go)) | — | — | single |
| `workspaces members invite` | `POST /workspaces/{workspace_id}/members` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/invitation.go)) | — | `--email`, `--role` | none |
| `workspaces members update` | `PATCH /workspaces/{workspace_id}/members/{member_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace.go)) | — | `--role` | none |
| `workspaces members remove` | `DELETE /workspaces/{workspace_id}/members/{member_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace.go)) | — | — | none |
| `workspaces invitations list` | `GET /workspaces/{workspace_id}/invitations` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/invitation.go)) | — | — | single |
| `workspaces invitations revoke` | `DELETE /workspaces/{workspace_id}/invitations/{invitation_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/invitation.go)) | — | — | none |
| `runtime-profiles list` | `GET /workspaces/{workspace_id}/runtime-profiles` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime_profile.go)) | — | — | single |
| `runtime-profiles create` | `POST /workspaces/{workspace_id}/runtime-profiles` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime_profile.go)) | — | `--display-name`, `--description`, `--protocol-family`, `--runtime-type`, `--command-name`, `--enabled` | none |
| `runtime-profiles update` | `PATCH /workspaces/{workspace_id}/runtime-profiles/{profile_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime_profile.go)) | — | `--display-name`, `--description`, `--protocol-family`, `--runtime-type`, `--command-name`, `--enabled` | none |
| `runtime-profiles delete` | `DELETE /workspaces/{workspace_id}/runtime-profiles/{profile_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime_profile.go)) | — | — | none |
| `runtime-profiles get` | `GET /workspaces/{workspace_id}/runtime-profiles/{profile_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime_profile.go)) | — | — | none |
| `mcp-servers list` | `GET /workspaces/{workspace_id}/mcp-servers` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace_mcp_api.go)) | — | — | single |
| `mcp-servers create` | `POST /workspaces/{workspace_id}/mcp-servers` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace_mcp_api.go)) | — | `--name` | none |
| `mcp-servers update` | `PUT /workspaces/{workspace_id}/mcp-servers/{server_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace_mcp_api.go)) | — | `--name` | none |
| `mcp-servers delete` | `DELETE /workspaces/{workspace_id}/mcp-servers/{server_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/workspace_mcp_api.go)) | — | — | none |

### Catalogs

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `labels list` | `GET /labels` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | `--resource-type` | — | single |
| `labels create` | `POST /labels` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | `--name`, `--color`, `--description` | none |
| `labels update` | `PUT /labels/{label_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | `--name`, `--color`, `--description` | none |
| `labels get` | `GET /labels/{label_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | — | none |
| `labels delete` | `DELETE /labels/{label_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | — | none |
| `properties list` | `GET /properties` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/property.go)) | `--include-archived` | — | single |
| `properties create` | `POST /properties` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/property.go)) | — | `--name` | none |
| `properties update` | `PATCH /properties/{property_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/property.go)) | — | `--name` | none |
| `properties get` | `GET /properties/{property_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/property.go)) | — | — | none |
| `issue-statuses list` | `GET /issue-statuses` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_status.go)) | `--include-archived` | — | single |
| `issue-statuses create` | `POST /issue-statuses` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_status.go)) | — | `--name`, `--category`, `--color` | none |
| `issue-statuses update` | `PATCH /issue-statuses/{status_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_status.go)) | — | `--name`, `--color`, `--description`, `--icon`, `--position` | none |
| `issue-statuses archive` | `DELETE /issue-statuses/{status_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_status.go)) | — | — | none |
| `skills list` | `GET /skills` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | `--include` | — | single |
| `skills create` | `POST /skills` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | `--name`, `--description` | none |
| `skills update` | `PUT /skills/{skill_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | `--name`, `--description` | none |
| `skills get` | `GET /skills/{skill_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | `--include` | — | none |
| `skills delete` | `DELETE /skills/{skill_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | — | none |
| `issue-statuses reorder` | `PATCH /issue-statuses/reorder` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_status.go)) | — | — | none |
| `skills search` | `GET /skills/search` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | `--q` | — | single |
| `skills import` | `POST /skills/import` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | — | none |
| `skills refresh` | `POST /skills/{skill_id}/refresh` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill_refresh.go)) | — | — | none |
| `agents labels list` | `GET /agents/{agent_id}/labels` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | — | single |
| `agents labels add` | `POST /agents/{agent_id}/labels` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | `--label-id` | none |
| `agents labels remove` | `DELETE /agents/{agent_id}/labels/{label_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | — | none |
| `skills labels list` | `GET /skills/{skill_id}/labels` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | — | single |
| `skills labels add` | `POST /skills/{skill_id}/labels` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | `--label-id` | none |
| `skills labels remove` | `DELETE /skills/{skill_id}/labels/{label_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/label.go)) | — | — | none |
| `skills files list` | `GET /skills/{skill_id}/files` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | `--include` | — | single |
| `skills files set` | `PUT /skills/{skill_id}/files` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | — | none |
| `skills files delete` | `DELETE /skills/{skill_id}/files/{file_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/skill.go)) | — | — | none |

### Tasks

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `tasks messages` | `GET /tasks/{task_id}/messages` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/daemon.go)) | `--since` | — | single |
| `tasks cancel` | `POST /tasks/{task_id}/cancel` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/chat.go)) | — | — | none |
| `runtimes list` | `GET /runtimes` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime.go)) | `--owner` | — | single |
| `runtimes update` | `PATCH /runtimes/{runtime_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime.go)) | — | `--custom-name`, `--visibility`, `--apply-to-machine` | none |
| `runtimes delete` | `DELETE /runtimes/{runtime_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime.go)) | — | — | none |
| `runtimes usage` | `GET /runtimes/{runtime_id}/usage` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime.go)) | — | — | none |
| `runtimes usage-by-agent` | `GET /runtimes/{runtime_id}/usage/by-agent` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime.go)) | — | — | none |
| `runtimes usage-by-hour` | `GET /runtimes/{runtime_id}/usage/by-hour` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime.go)) | — | — | none |
| `runtimes activity` | `GET /runtimes/{runtime_id}/activity` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/runtime.go)) | — | — | none |

### Scheduling

| Command | HTTP endpoint (relative to `/api`) | Query flags | Body flags | Paging |
| --- | --- | --- | --- | --- |
| `autopilots list` | `GET /autopilots` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | `--status` | — | single |
| `autopilots get` | `GET /autopilots/{autopilot_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | — | none |
| `autopilots create` | `POST /autopilots` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | `--title`, `--description`, `--project-id`, `--assignee-type`, `--assignee-id`, `--execution-mode`, `--issue-title-template` | none |
| `autopilots update` | `PATCH /autopilots/{autopilot_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | `--title`, `--description`, `--project-id`, `--assignee-type`, `--assignee-id`, `--execution-mode`, `--issue-title-template`, `--status` | none |
| `autopilots delete` | `DELETE /autopilots/{autopilot_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | — | none |
| `autopilots trigger` | `POST /autopilots/{autopilot_id}/trigger` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | — | none |
| `autopilots cron-preview` | `GET /autopilots/cron-preview` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot_cron_preview.go)) | `--expr`, `--tz` | — | none |
| `autopilots runs list` | `GET /autopilots/{autopilot_id}/runs` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | — | offsetuntilshort |
| `autopilots runs get` | `GET /autopilots/{autopilot_id}/runs/{run_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | — | none |
| `autopilots triggers create` | `POST /autopilots/{autopilot_id}/triggers` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | `--cron-expression`, `--timezone` | none |
| `autopilots triggers update` | `PATCH /autopilots/{autopilot_id}/triggers/{trigger_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | `--cron-expression`, `--timezone` | none |
| `autopilots triggers delete` | `DELETE /autopilots/{autopilot_id}/triggers/{trigger_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/autopilot.go)) | — | — | none |
| `issues wakeups list` | `GET /issues/{issue_id}/wakeups` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | — | single |
| `issues wakeups create` | `POST /issues/{issue_id}/wakeups` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | — | none |
| `issues wakeups set` | `PUT /issues/{issue_id}/wakeups/{wakeup_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | — | none |
| `issues wakeups edit-instruction` | `PATCH /issues/{issue_id}/wakeups/{wakeup_id}/instruction` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | `--instruction` | none |
| `issues wakeups enable` | `POST /issues/{issue_id}/wakeups/{wakeup_id}/enable` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | — | none |
| `issues wakeups disable` | `POST /issues/{issue_id}/wakeups/{wakeup_id}/disable` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | — | none |
| `issues wakeups delete` | `DELETE /issues/{issue_id}/wakeups/{wakeup_id}` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | — | none |
| `issues wakeups trigger` | `POST /issues/{issue_id}/wakeups/{wakeup_id}/trigger` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | — | none |
| `issues wakeups runs` | `GET /issues/{issue_id}/wakeups/{wakeup_id}/runs` ([source](https://github.com/multica-ai/multica/blob/83527872634824012dd26bb183aa216fab631c05/server/internal/handler/issue_wakeup.go)) | — | — | capped |

## Generic request

```sh
aai-cli --profile multica multica request get issues --query status=STATUS_KEY
aai-cli --profile multica multica request post issues --allow-write --json issue.json
```

This command uses the existing generic-request interface, returns one page, and does not follow redirects. Header continuations are exposed in `_aai.pagination`. No arbitrary HTTP headers or alternate authentication scheme are introduced.

## Common JSON bodies

For nested fields, use a local JSON file or stdin. Examples contain placeholders, not working IDs or credentials.

| Operation | Example JSON body |
| --- | --- |
| Clear an issue assignment | `{"assignee_type":null,"assignee_id":null}` |
| Add/replace agent skills | `{"skill_ids":["SKILL_UUID"]}` |
| Set agent environment | `{"custom_env":{"KEY":"VALUE"}}` (use a local file; do not echo secrets) |
| Create a workspace MCP server | `{"name":"Tools","config":{}}` (fill the supported provider configuration) |
| Create a skill | `{"name":"Review","description":"Review changes","content":"# Review\nInspect regressions."}` |
| Set issue metadata | `{"value":{"checked":true}}` |
| Edit wakeup instruction | `{"instruction":"New instruction","expected_instruction":"Prior instruction","revision":1}` |

Use [workflows](workflows.md) for comment pagination rules, inbox semantics, discovery, assignment, and scheduling.
