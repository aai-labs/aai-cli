# Jira

## CLI Scope

Jira commands cover issues, Jira Product Discovery ideas, projects, Agile boards, sprints, comments, and attachments. Use the full command reference for exact flags:

- [aai-cli command reference](../aai-cli-command-reference.md#jira)
- [Auth matrix](../auth-matrix.md#atlassian-cloud)

## Jira Product Discovery

Product Discovery ideas are Jira issues in `product_discovery`-type projects, managed through the Jira platform REST API with the same site and credentials. `jira ideas` commands scope searches to Product Discovery projects and discover project-specific idea fields (for example Impact or Effort custom fields) via the create-meta endpoints. Set idea custom fields by passing `fields.customfield_*` entries in `--json`; discover their ids with `jira ideas fields PROJECT`. Explicitly requested custom fields are preserved in `ideas list` results.

Use `jira ideas parse-url URL` to distinguish a Product Discovery saved-view URL from an existing idea. The opaque `/ideas/view/VIEW_ID` path value is view context; the optional `selectedIssue` query parameter is the Jira issue key. Use `jira ideas transitions` to list and perform workflow status transitions by id or name. Votes, reactions, insights, formula values, and saved-view configuration are not exposed by Atlassian's public APIs.

## Original API Docs

- [Jira Cloud REST API v3](https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/)
- [Jira Software Cloud REST API](https://developer.atlassian.com/cloud/jira/software/rest/intro/)
- [Jira Product Discovery ideas over the Jira platform API](https://community.atlassian.com/forums/Jira-Product-Discovery/Product-Discovery-APIs/td-p/2523303)
- [Atlassian API tokens](https://support.atlassian.com/atlassian-account/docs/manage-api-tokens-for-your-atlassian-account/)
- [Atlassian Document Format](https://developer.atlassian.com/cloud/jira/platform/apis/document/structure/)

## Local API Snapshots

- `docs/atlassian/jira/openapi.json`
- `docs/atlassian/adf.md`
