# Confluence

## CLI Scope

Confluence commands cover spaces, pages, search, page moves, comments, attachments, page versions, page permissions, and experimental database reads. Use the full command reference for exact flags:

- [aai-cli command reference](../aai-cli-command-reference.md#confluence)
- [Auth matrix](../auth-matrix.md#atlassian-cloud)

## Original API Docs

- [Confluence Cloud REST API v2](https://developer.atlassian.com/cloud/confluence/rest/v2/intro/)
- [Atlassian API tokens](https://support.atlassian.com/atlassian-account/docs/manage-api-tokens-for-your-atlassian-account/)

## Local API Snapshots

- `docs/atlassian/confluence/openapi.json`

Database row reads and field/view listings are experimental in the current [Atlassian database API](https://developer.atlassian.com/cloud/confluence/rest/v2/api-group-database/). Database entry, field, and view writes are not documented by Atlassian.
