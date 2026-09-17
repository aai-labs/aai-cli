# ADR 0002: Microsoft Graph Excel and Word file boundaries

- Status: Accepted
- Date: 2026-09-16

## Context

Microsoft Graph exposes Excel workbooks through typed workbook resources such as worksheets, ranges, tables, and table rows. Those operations are delegated-only and require `Files.ReadWrite`. Word documents are exposed through Graph as `driveItem` file content; Graph does not provide a comparable paragraph/table editing API.

The CLI already has generic OneDrive and SharePoint file transfer commands and a separate local Excel command. Adding a local Word editor or pretending that a remote Word API exists would create a second document model, risk lossy OOXML rewrites, and obscure the provider boundary.

## Decision

- Add a typed `microsoft excel` command group only for documented Graph workbook operations: worksheet management, range reads/updates/clears, table management, and table-row reads/appends.
- Require a delegated Microsoft profile for every typed workbook operation and reject application profiles before making a request.
- Address workbooks by drive item ID or drive-relative path, with an optional explicit drive ID for SharePoint or a non-default OneDrive. Resolve path targets to drive-item IDs before workbook calls because some SharePoint tenants reject a path directly on the workbook relationship.
- Keep Word documents and unsupported Excel features in the file-transfer layer. Agents download the bytes, use an external library/program for local editing, then upload the complete replacement.
- Explain whole-file replacement, version history, backups, concurrent edits, and external-library fidelity in the Microsoft documentation and bundled skill.
- Keep `microsoft request` as the escape hatch for actual Graph endpoints not yet typed; it is not a semantic Word editor.

## Alternatives considered

1. Add a `microsoft word` abstraction that downloads and edits documents internally. Rejected: it would invent a provider API and require the CLI to own a lossy or incomplete OOXML document model.
2. Add client-side Excel find/replace or batch transactions. Rejected for this release: those semantics are not a single documented workbook resource operation and can overwrite formulas or partially apply.
3. Use application credentials for Graph Excel. Rejected: Microsoft documents application permissions as unsupported for workbook APIs.
4. Replace the existing local Excel command with the Graph surface. Rejected: local files and remote workbooks have different ownership, auth, and failure semantics; preserve both explicit surfaces.

## Consequences

- Agents get direct, machine-readable Excel operations for OneDrive and SharePoint without downloading files.
- Word editing remains flexible because agents choose the appropriate external document library and fidelity level.
- Whole-file Word replacements can overwrite newer content and may lose unsupported document features; callers must use copies/version history when required.
- No credential, data, or deployment migration is needed. Removing the typed Excel module and docs is a safe rollback.
