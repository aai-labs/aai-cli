# GitLab

## CLI Scope

GitLab commands cover projects, merge requests, approvals, notes, discussions, issues, branches, commits, source files, pipelines, and job logs, on gitlab.com and self-hosted instances. Use the full command reference for exact flags:

- [aai-cli command reference](../aai-cli-command-reference.md#gitlab)
- [Auth matrix](../auth-matrix.md)

Profiles use `auth_type = "bearer_token"`, which takes personal, group, and project access tokens. `base_url` is the API root: it defaults to `https://gitlab.com/api/v4`, and a self-hosted instance sets `https://<host>/api/v4`.

Lists walk `page=N` rather than following `Link` headers, because those URLs name the upstream host and would bypass the credential gateway.

## Original API Docs

- [GitLab REST API](https://docs.gitlab.com/api/rest/)
- [Personal access tokens](https://docs.gitlab.com/user/profile/personal_access_tokens/)
