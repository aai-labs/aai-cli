---
name: aai-sharepoint
description: Use aai-cli to browse SharePoint sites, document libraries, and files through Microsoft Graph — list and read items, download and upload file content, and track drive changes with delta.
---

# aai-cli SharePoint

Use this skill when working with SharePoint document libraries through `aai-cli sharepoint`.

Before running commands, confirm the active profile or pass `--profile`. SharePoint profiles authenticate as the organisation's **app** (`auth_type = "microsoft_client_credentials"`), not as a person. The app can open only the sites its administrator explicitly granted it.

**Start from the configured site URLs** — your instructions list them. `sites list` cannot discover granted sites and normally returns 403; do not treat that as a broken setup. `sites get <url>` resolves a site (it accepts a plain SharePoint URL), `drives list` gives that site's document libraries, then `items list` walks a drive. Item ids come from `items list`, not from the web UI.

A 403 on a specific site means that site has not been granted to the app — tell the user which site, rather than retrying. `items upload` also needs the site to be granted with write access, and requires `--allow-write` because it overwrites remote content. To edit a spreadsheet, chain this with the `aai-excel` skill: `items download`, edit the local `.xlsx`, then `items upload`.

Use `items delta` rather than re-listing a whole drive when you only need what changed; keep the `@odata.deltaLink` token it returns and pass it back as `--token` next time.

Successful output is JSON on stdout. Errors are structured JSON on stderr. See [the command reference](references/command-reference.md) for command shapes, response notes, and pagination behaviour.
