---
name: live-crud-e2e
description: Design and run safe, repeatable live create/read/update/delete checks for aai-cli integrations, including direct and managed-gateway parity.
---

# Live CRUD checks

Use this skill when verifying that an aai-cli integration can write and read back provider data. Follow `TESTING.md` for repository conventions.

- Run live mutations only when the user has authorized testing against that provider and project. Confirm the target profile and a disposable resource scope first; never change pre-existing customer records.
- Make each run independent: generate a unique marker, create the smallest disposable record, read and verify it, update one harmless field, read and verify the change, delete it, then verify it is gone. Do not rely on records from previous runs. This makes the *test* rerunnable; create and delete API calls themselves need not be idempotent.
- Capture the created ID immediately. Attempt cleanup after any later failure, including assertion failures. If gateway cleanup fails, try a separately authorized direct cleanup and report both failures. Do not retry a write blindly after an ambiguous network result; locate the unique marker first.
- Exercise the same typed commands in direct and gateway modes with distinct disposable records. Grant only the operations and methods under test. Check externally visible results, structured errors, and absence of credential material; do not assert only internal calls.
- Keep live tests ignored by default. Load credentials from ignored `local/` configuration or environment variables, never print or commit them. Report the tested instance, commit, commands, results, cleanup outcome, and any untested operations without provider payloads or secrets.
