# Agent Barn command reference

Generated from the pinned OpenAPI contract. Prefix commands with `aai-cli --profile agent-barn agent-barn`. Scoped commands require `--organization-id` or `profile.org`. All path and query fields use hyphenated flags; bodies accept `--json FILE` or `--json -`. Scalar body flags override JSON fields. Use command `--help` for required flags and accepted values.

Collections use `--limit 100` by default and `--page-skip` for exact continuation. Download and stream commands require `--output`; streams default to `--duration-seconds 30`.

| Command | Method and API-relative path | Query flags |
|---|---|---|
| `auth context` | `GET /auth/context` |  |
| `auth me get` | `GET /auth/me` |  |
| `auth me update` | `POST /auth/me` |  |
| `auth me change-password` | `POST /auth/me/change-password` |  |
| `discovery` | `GET /discovery` |  |
| `health` | `GET /health` |  |
| `integrations google authorize-url get` | `GET /integrations/google/authorize-url` | `--client-id`, `--provider`, `--services`, `--read-only` |
| `integrations google token create` | `POST /integrations/google/token` |  |
| `organizations create` | `POST /organizations` |  |
| `organizations delete` | `DELETE /organizations/{organization_id}` |  |
| `organizations get` | `GET /organizations/{organization_id}` |  |
| `organizations update` | `PATCH /organizations/{organization_id}` |  |
| `agent-overview list` | `GET /organizations/{organization_id}/agent-overview` | `--period`, `--from-date`, `--to-date`, `--granularity` |
| `agent-settings get` | `GET /organizations/{organization_id}/agent-settings` |  |
| `agent-settings set` | `PUT /organizations/{organization_id}/agent-settings` |  |
| `agents list` | `GET /organizations/{organization_id}/agents` | `--page`, `--page-size`, `--status` |
| `agents create` | `POST /organizations/{organization_id}/agents` |  |
| `agents llm-budgets list` | `GET /organizations/{organization_id}/agents/llm-budgets` |  |
| `agents models get` | `GET /organizations/{organization_id}/agents/models` | `--catalog` |
| `agents name-suggestion get` | `GET /organizations/{organization_id}/agents/name-suggestion` |  |
| `agents share-roles list` | `GET /organizations/{organization_id}/agents/share-roles` |  |
| `agents delete` | `DELETE /organizations/{organization_id}/agents/{agent_id}` |  |
| `agents get` | `GET /organizations/{organization_id}/agents/{agent_id}` |  |
| `agents update` | `PATCH /organizations/{organization_id}/agents/{agent_id}` |  |
| `agents activity get` | `GET /organizations/{organization_id}/agents/{agent_id}/activity` | `--period`, `--from-date`, `--to-date`, `--granularity` |
| `agents activity calls list` | `GET /organizations/{organization_id}/agents/{agent_id}/activity/calls` | `--page`, `--page-size`, `--period`, `--from-date`, `--to-date`, `--granularity`, `--trigger` |
| `agents activity wakes list` | `GET /organizations/{organization_id}/agents/{agent_id}/activity/wakes` | `--page`, `--page-size`, `--period`, `--from-date`, `--to-date`, `--granularity`, `--trigger` |
| `agents configuration get` | `GET /organizations/{organization_id}/agents/{agent_id}/configuration` |  |
| `agents configuration draft update` | `PATCH /organizations/{organization_id}/agents/{agent_id}/configuration/draft` |  |
| `agents configuration draft create` | `POST /organizations/{organization_id}/agents/{agent_id}/configuration/draft` |  |
| `agents configuration draft publish` | `POST /organizations/{organization_id}/agents/{agent_id}/configuration/draft/publish` |  |
| `agents configuration select` | `POST /organizations/{organization_id}/agents/{agent_id}/configuration/select` |  |
| `agents connection-directory-preview create` | `POST /organizations/{organization_id}/agents/{agent_id}/connection-directory-preview` |  |
| `agents connections list` | `GET /organizations/{organization_id}/agents/{agent_id}/connections` |  |
| `agents connections create` | `POST /organizations/{organization_id}/agents/{agent_id}/connections` |  |
| `agents connections delete` | `DELETE /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}` | `--revision` |
| `agents connections update` | `PATCH /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}` |  |
| `agents connections app-package` | `GET /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}/app-package` |  |
| `agents connections deliveries retry` | `POST /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}/deliveries/{delivery_id}/retry` |  |
| `agents connections directory list` | `GET /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}/directory/{kind}` | `--search`, `--guild-id` |
| `agents connections install-link get` | `GET /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}/install-link` |  |
| `agents connections journal list` | `GET /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}/journal` | `--page`, `--page-size`, `--kind`, `--since`, `--until`, `--stage`, `--failed-only`, `--retryable-only`, `--direction`, `--delivery-id`, `--order` |
| `agents connections reconnect` | `POST /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}/reconnect` |  |
| `agents connections summary get` | `GET /organizations/{organization_id}/agents/{agent_id}/connections/{connection_id}/summary` | `--since`, `--until`, `--window-minutes` |
| `agents conversations channels list` | `GET /organizations/{organization_id}/agents/{agent_id}/conversations/channels` |  |
| `agents conversations connections channels messages list` | `GET /organizations/{organization_id}/agents/{agent_id}/conversations/connections/{connection_id}/channels/{channel_id}/messages` | `--page-size`, `--from-date`, `--to-date`, `--before-occurred-at`, `--before-id` |
| `agents diagnostics get` | `GET /organizations/{organization_id}/agents/{agent_id}/diagnostics` |  |
| `agents healthz get` | `GET /organizations/{organization_id}/agents/{agent_id}/healthz` |  |
| `agents integrations sharepoint get` | `GET /organizations/{organization_id}/agents/{agent_id}/integrations/sharepoint` |  |
| `agents integrations sharepoint authorize-url get` | `GET /organizations/{organization_id}/agents/{agent_id}/integrations/sharepoint/authorize-url` | `--connection-id`, `--read-only`, `--mode`, `--sites`, `--remove-all` |
| `agents integrations sharepoint setup get` | `GET /organizations/{organization_id}/agents/{agent_id}/integrations/sharepoint/setup` | `--connection-id` |
| `agents integrations sharepoint sign-in` | `POST /organizations/{organization_id}/agents/{agent_id}/integrations/sharepoint/sign-in` |  |
| `agents integrations isolation set` | `PUT /organizations/{organization_id}/agents/{agent_id}/integrations/{provider}/isolation` |  |
| `agents integrations validate` | `POST /organizations/{organization_id}/agents/{agent_id}/integrations/{provider}/validate` |  |
| `agents llm-budget get` | `GET /organizations/{organization_id}/agents/{agent_id}/llm-budget` |  |
| `agents llm-budget set` | `PUT /organizations/{organization_id}/agents/{agent_id}/llm-budget` |  |
| `agents logs get` | `GET /organizations/{organization_id}/agents/{agent_id}/logs` | `--tail-lines` |
| `agents logs history list` | `GET /organizations/{organization_id}/agents/{agent_id}/logs/history` | `--snapshot-id` |
| `agents logs stream` | `GET /organizations/{organization_id}/agents/{agent_id}/logs/stream` | `--tail-lines` |
| `agents managed-update` | `POST /organizations/{organization_id}/agents/{agent_id}/managed-update` |  |
| `agents memory set` | `PUT /organizations/{organization_id}/agents/{agent_id}/memory` |  |
| `agents memory items list` | `GET /organizations/{organization_id}/agents/{agent_id}/memory/items` | `--search`, `--page`, `--page-size` |
| `agents resource-usage get` | `GET /organizations/{organization_id}/agents/{agent_id}/resource-usage` | `--range` |
| `agents restore-points list` | `GET /organizations/{organization_id}/agents/{agent_id}/restore-points` | `--page`, `--page-size` |
| `agents restore-points create` | `POST /organizations/{organization_id}/agents/{agent_id}/restore-points` |  |
| `agents restore-points delete` | `DELETE /organizations/{organization_id}/agents/{agent_id}/restore-points/{restore_point_id}` |  |
| `agents restore-points get` | `GET /organizations/{organization_id}/agents/{agent_id}/restore-points/{restore_point_id}` |  |
| `agents restore-points configuration create` | `POST /organizations/{organization_id}/agents/{agent_id}/restore-points/{restore_point_id}/configuration` |  |
| `agents restore-points restore` | `POST /organizations/{organization_id}/agents/{agent_id}/restore-points/{restore_point_id}/restore` |  |
| `agents share get` | `GET /organizations/{organization_id}/agents/{agent_id}/share` |  |
| `agents share set` | `PUT /organizations/{organization_id}/agents/{agent_id}/share` |  |
| `agents skills list` | `GET /organizations/{organization_id}/agents/{agent_id}/skills` | `--page`, `--page-size`, `--search`, `--source` |
| `agents skills create` | `POST /organizations/{organization_id}/agents/{agent_id}/skills` |  |
| `agents skills delete` | `DELETE /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}` |  |
| `agents skills update` | `PATCH /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}` |  |
| `agents skills draft delete` | `DELETE /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/draft` |  |
| `agents skills draft get` | `GET /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/draft` |  |
| `agents skills draft update` | `PATCH /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/draft` |  |
| `agents skills draft create` | `POST /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/draft` |  |
| `agents skills draft publish` | `POST /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/draft/publish` |  |
| `agents skills files get` | `GET /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/files` |  |
| `agents skills fork` | `POST /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/fork` |  |
| `agents skills source-update` | `POST /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/source-update` |  |
| `agents skills versions list` | `GET /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/versions` |  |
| `agents skills versions delete` | `DELETE /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/versions/{version}` |  |
| `agents skills versions get` | `GET /organizations/{organization_id}/agents/{agent_id}/skills/{skill_id}/versions/{version}` |  |
| `agents start` | `POST /organizations/{organization_id}/agents/{agent_id}/start` |  |
| `agents stop` | `POST /organizations/{organization_id}/agents/{agent_id}/stop` |  |
| `agents template get` | `GET /organizations/{organization_id}/agents/{agent_id}/template/{version}` |  |
| `agents tool-calls list` | `GET /organizations/{organization_id}/agents/{agent_id}/tool-calls` | `--page`, `--page-size`, `--tool-name`, `--status`, `--from-date`, `--to-date` |
| `agents web-chat messages list` | `GET /organizations/{organization_id}/agents/{agent_id}/web-chat/messages` | `--thread-id`, `--after-id` |
| `agents web-chat messages create` | `POST /organizations/{organization_id}/agents/{agent_id}/web-chat/messages` |  |
| `agents web-chat stream` | `GET /organizations/{organization_id}/agents/{agent_id}/web-chat/stream` | `--thread-id` |
| `agents web-chat threads list` | `GET /organizations/{organization_id}/agents/{agent_id}/web-chat/threads` |  |
| `agents web-chat threads delete` | `DELETE /organizations/{organization_id}/agents/{agent_id}/web-chat/threads/{thread_id}` |  |
| `agents web-chat threads update` | `PATCH /organizations/{organization_id}/agents/{agent_id}/web-chat/threads/{thread_id}` |  |
| `agents web-chat threads stop` | `POST /organizations/{organization_id}/agents/{agent_id}/web-chat/threads/{thread_id}/stop` |  |
| `agents webhooks list` | `GET /organizations/{organization_id}/agents/{agent_id}/webhooks` |  |
| `agents webhooks create` | `POST /organizations/{organization_id}/agents/{agent_id}/webhooks` |  |
| `agents webhooks delivery-platforms list` | `GET /organizations/{organization_id}/agents/{agent_id}/webhooks/delivery-platforms` |  |
| `agents webhooks delete` | `DELETE /organizations/{organization_id}/agents/{agent_id}/webhooks/{webhook_id}` | `--revision` |
| `agents webhooks get` | `GET /organizations/{organization_id}/agents/{agent_id}/webhooks/{webhook_id}` |  |
| `agents webhooks update` | `PATCH /organizations/{organization_id}/agents/{agent_id}/webhooks/{webhook_id}` |  |
| `agents webhooks invocations list` | `GET /organizations/{organization_id}/agents/{agent_id}/webhooks/{webhook_id}/invocations` | `--page`, `--page-size` |
| `agents webhooks invocations retry` | `POST /organizations/{organization_id}/agents/{agent_id}/webhooks/{webhook_id}/invocations/{invocation_id}/retry` |  |
| `agents webhooks rotate-secret` | `POST /organizations/{organization_id}/agents/{agent_id}/webhooks/{webhook_id}/rotate-secret` | `--revision` |
| `communication-platforms list` | `GET /organizations/{organization_id}/communication-platforms` |  |
| `costs list` | `GET /organizations/{organization_id}/costs` | `--page`, `--page-size`, `--period`, `--from-date`, `--to-date`, `--granularity`, `--agent-id`, `--model`, `--search`, `--sort` |
| `costs agents list` | `GET /organizations/{organization_id}/costs/agents` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--agent-id`, `--model`, `--search`, `--sort` |
| `costs agents get` | `GET /organizations/{organization_id}/costs/agents/{agent_id}` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--model`, `--search`, `--sort` |
| `costs agents calls list` | `GET /organizations/{organization_id}/costs/agents/{agent_id}/calls` | `--page`, `--page-size`, `--period`, `--from-date`, `--to-date`, `--granularity`, `--model`, `--search`, `--sort` |
| `costs agents filters models list` | `GET /organizations/{organization_id}/costs/agents/{agent_id}/filters/models` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--model`, `--search`, `--sort` |
| `costs agents monthly list` | `GET /organizations/{organization_id}/costs/agents/{agent_id}/monthly` | `--months`, `--model`, `--search`, `--sort` |
| `costs filters agents list` | `GET /organizations/{organization_id}/costs/filters/agents` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--agent-id`, `--model`, `--search`, `--sort` |
| `costs filters models list` | `GET /organizations/{organization_id}/costs/filters/models` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--agent-id`, `--model`, `--search`, `--sort` |
| `costs monthly list` | `GET /organizations/{organization_id}/costs/monthly` | `--months`, `--agent-id`, `--model`, `--search`, `--sort` |
| `costs summary get` | `GET /organizations/{organization_id}/costs/summary` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--agent-id`, `--model`, `--search`, `--sort` |
| `llm-budget get` | `GET /organizations/{organization_id}/llm-budget` |  |
| `llm-budget set` | `PUT /organizations/{organization_id}/llm-budget` |  |
| `organizations members list` | `GET /organizations/{organization_id}/members` | `--search`, `--limit` |
| `organizations members create` | `POST /organizations/{organization_id}/members` |  |
| `organizations members delete` | `DELETE /organizations/{organization_id}/members/{user_id}` |  |
| `organizations members update` | `PATCH /organizations/{organization_id}/members/{user_id}` |  |
| `organizations members resend-invite` | `POST /organizations/{organization_id}/members/{user_id}/resend-invite` |  |
| `memory-grants list` | `GET /organizations/{organization_id}/memory-grants` |  |
| `memory-grants create` | `POST /organizations/{organization_id}/memory-grants` |  |
| `memory-grants delete` | `DELETE /organizations/{organization_id}/memory-grants/{grant_id}` |  |
| `memory items list` | `GET /organizations/{organization_id}/memory/items` | `--search`, `--page`, `--page-size` |
| `shared-credentials list` | `GET /organizations/{organization_id}/shared-credentials` | `--page`, `--page-size`, `--search`, `--provider` |
| `shared-credentials create` | `POST /organizations/{organization_id}/shared-credentials` |  |
| `shared-credentials briefs list` | `GET /organizations/{organization_id}/shared-credentials/briefs` |  |
| `shared-credentials delete` | `DELETE /organizations/{organization_id}/shared-credentials/{credential_id}` |  |
| `shared-credentials get` | `GET /organizations/{organization_id}/shared-credentials/{credential_id}` |  |
| `shared-credentials update` | `PATCH /organizations/{organization_id}/shared-credentials/{credential_id}` |  |
| `shared-credentials validate` | `POST /organizations/{organization_id}/shared-credentials/{credential_id}/validate` |  |
| `skills list` | `GET /organizations/{organization_id}/skills` | `--page`, `--page-size`, `--search`, `--source` |
| `skills create` | `POST /organizations/{organization_id}/skills` |  |
| `skills delete` | `DELETE /organizations/{organization_id}/skills/{skill_id}` |  |
| `skills get` | `GET /organizations/{organization_id}/skills/{skill_id}` |  |
| `skills update` | `PATCH /organizations/{organization_id}/skills/{skill_id}` |  |
| `skills draft delete` | `DELETE /organizations/{organization_id}/skills/{skill_id}/draft` |  |
| `skills draft get` | `GET /organizations/{organization_id}/skills/{skill_id}/draft` |  |
| `skills draft update` | `PATCH /organizations/{organization_id}/skills/{skill_id}/draft` |  |
| `skills draft create` | `POST /organizations/{organization_id}/skills/{skill_id}/draft` |  |
| `skills draft publish` | `POST /organizations/{organization_id}/skills/{skill_id}/draft/publish` |  |
| `skills files get` | `GET /organizations/{organization_id}/skills/{skill_id}/files` |  |
| `skills fork` | `POST /organizations/{organization_id}/skills/{skill_id}/fork` |  |
| `skills source-update` | `POST /organizations/{organization_id}/skills/{skill_id}/source-update` |  |
| `skills versions list` | `GET /organizations/{organization_id}/skills/{skill_id}/versions` |  |
| `skills versions delete` | `DELETE /organizations/{organization_id}/skills/{skill_id}/versions/{version}` |  |
| `skills versions get` | `GET /organizations/{organization_id}/skills/{skill_id}/versions/{version}` |  |
| `templates list` | `GET /organizations/{organization_id}/templates` | `--page`, `--page-size`, `--search`, `--source` |
| `templates create` | `POST /organizations/{organization_id}/templates` |  |
| `templates lineages list` | `GET /organizations/{organization_id}/templates/lineages` |  |
| `templates delete` | `DELETE /organizations/{organization_id}/templates/{template_key}` |  |
| `templates get` | `GET /organizations/{organization_id}/templates/{template_key}` |  |
| `templates draft delete` | `DELETE /organizations/{organization_id}/templates/{template_key}/draft` |  |
| `templates draft get` | `GET /organizations/{organization_id}/templates/{template_key}/draft` |  |
| `templates draft update` | `PATCH /organizations/{organization_id}/templates/{template_key}/draft` |  |
| `templates draft create` | `POST /organizations/{organization_id}/templates/{template_key}/draft` | `--source-version`, `--source-scope` |
| `templates draft publish` | `POST /organizations/{organization_id}/templates/{template_key}/draft/publish` |  |
| `templates platform-update` | `POST /organizations/{organization_id}/templates/{template_key}/platform-update` |  |
| `templates versions list` | `GET /organizations/{organization_id}/templates/{template_key}/versions` | `--include-platform` |
| `organizations transfer-ownership` | `POST /organizations/{organization_id}/transfer-ownership` |  |
| `value get` | `GET /organizations/{organization_id}/value` | `--period`, `--from-date`, `--to-date`, `--granularity` |
| `value-settings get` | `GET /organizations/{organization_id}/value-settings` |  |
| `value-settings set` | `PUT /organizations/{organization_id}/value-settings` |  |
| `value activity get` | `GET /organizations/{organization_id}/value/activity` | `--period`, `--from-date`, `--to-date`, `--granularity` |
| `platform costs list` | `GET /platform/costs` | `--page`, `--page-size`, `--period`, `--from-date`, `--to-date`, `--granularity`, `--organization-id`, `--agent-id`, `--model`, `--search`, `--sort` |
| `platform costs filters agents list` | `GET /platform/costs/filters/agents` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--organization-id`, `--agent-id`, `--model`, `--search`, `--sort` |
| `platform costs filters models list` | `GET /platform/costs/filters/models` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--organization-id`, `--agent-id`, `--model`, `--search`, `--sort` |
| `platform costs monthly list` | `GET /platform/costs/monthly` | `--months`, `--organization-id`, `--agent-id`, `--model`, `--search`, `--sort` |
| `platform costs organizations list` | `GET /platform/costs/organizations` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--organization-id`, `--agent-id`, `--model`, `--search`, `--sort` |
| `platform costs summary get` | `GET /platform/costs/summary` | `--period`, `--from-date`, `--to-date`, `--granularity`, `--organization-id`, `--agent-id`, `--model`, `--search`, `--sort` |
| `platform event-deliveries list` | `GET /platform/event-deliveries` | `--page`, `--page-size`, `--search`, `--status`, `--organization-id`, `--event-name`, `--created-from`, `--created-to`, `--sort` |
| `platform event-deliveries event-types list` | `GET /platform/event-deliveries/event-types` |  |
| `platform event-deliveries summary get` | `GET /platform/event-deliveries/summary` |  |
| `platform organizations list` | `GET /platform/organizations` | `--page`, `--page-size`, `--search` |
| `platform organizations get` | `GET /platform/organizations/{organization_id}` |  |
| `platform organizations llm-budget set` | `PUT /platform/organizations/{organization_id}/llm-budget` |  |
| `platform organizations llm-budget coverage get` | `GET /platform/organizations/{organization_id}/llm-budget/coverage` |  |
| `platform organizations llm-budget enroll create` | `POST /platform/organizations/{organization_id}/llm-budget/enroll` |  |
| `platform organizations members list` | `GET /platform/organizations/{organization_id}/members` | `--search`, `--page`, `--page-size` |
| `platform resource-limits set` | `PUT /platform/resource-limits` |  |
| `platform resource-usage get` | `GET /platform/resource-usage` | `--range`, `--organization-id` |
| `platform resource-usage agents get` | `GET /platform/resource-usage/agents/{agent_id}` |  |
| `platform settings agent-memory get` | `GET /platform/settings/agent-memory` |  |
| `platform settings agent-memory set` | `PUT /platform/settings/agent-memory` |  |
| `platform settings agent-memory models get` | `GET /platform/settings/agent-memory/models` |  |
| `platform skills list` | `GET /platform/skills` |  |
| `platform skills create` | `POST /platform/skills` |  |
| `platform skills delete` | `DELETE /platform/skills/{skill_id}` |  |
| `platform skills update` | `PATCH /platform/skills/{skill_id}` |  |
| `platform skills draft delete` | `DELETE /platform/skills/{skill_id}/draft` |  |
| `platform skills draft get` | `GET /platform/skills/{skill_id}/draft` |  |
| `platform skills draft update` | `PATCH /platform/skills/{skill_id}/draft` |  |
| `platform skills draft create` | `POST /platform/skills/{skill_id}/draft` |  |
| `platform skills draft publish` | `POST /platform/skills/{skill_id}/draft/publish` |  |
| `platform skills files get` | `GET /platform/skills/{skill_id}/files` |  |
| `platform skills versions list` | `GET /platform/skills/{skill_id}/versions` |  |
| `platform skills versions delete` | `DELETE /platform/skills/{skill_id}/versions/{version}` |  |
| `platform skills versions get` | `GET /platform/skills/{skill_id}/versions/{version}` |  |
| `platform stats agents get` | `GET /platform/stats/agents` | `--organization-id`, `--agent-id`, `--created-by-user-id`, `--platform`, `--period`, `--from-date`, `--to-date`, `--granularity` |
| `platform stats messages get` | `GET /platform/stats/messages` | `--organization-id`, `--agent-id`, `--created-by-user-id`, `--platform`, `--period`, `--from-date`, `--to-date`, `--granularity` |
| `platform templates list` | `GET /platform/templates` |  |
| `platform templates create` | `POST /platform/templates` |  |
| `platform templates get` | `GET /platform/templates/{template_key}` |  |
| `platform templates draft delete` | `DELETE /platform/templates/{template_key}/draft` |  |
| `platform templates draft get` | `GET /platform/templates/{template_key}/draft` |  |
| `platform templates draft update` | `PATCH /platform/templates/{template_key}/draft` |  |
| `platform templates draft create` | `POST /platform/templates/{template_key}/draft` | `--source-version` |
| `platform templates draft publish` | `POST /platform/templates/{template_key}/draft/publish` |  |
| `platform templates versions list` | `GET /platform/templates/{template_key}/versions` |  |
| `platform users list` | `GET /platform/users` | `--page`, `--page-size`, `--search`, `--organization-roles` |
| `platform users create` | `POST /platform/users` |  |
| `platform users get` | `GET /platform/users/{user_id}` |  |
| `platform users platform-privilege update` | `PATCH /platform/users/{user_id}/platform-privilege` |  |
| `platform users resend-invite` | `POST /platform/users/{user_id}/resend-invite` |  |
| `openapi` | `GET /openapi.json` |  |
