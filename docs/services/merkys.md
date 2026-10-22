# Merkys

## CLI Scope

Merkys commands let an agent ask the organization's Merkys knowledge base a question and get one grounded answer with cited sources, run deep research, and read conversations, documents, data sources, assistants, and (through `request`) the legal, medical, and case-law corpora. The integration is read and ask only: it never uploads, deletes, or changes documents, sources, users, or settings. Use the full command reference for exact flags:

- [aai-cli command reference](../aai-cli-command-reference.md#merkys)
- [Auth matrix](../auth-matrix.md#merkys)

## Original API Docs

Merkys is an in-house service with no public documentation. Its FastAPI OpenAPI document (`/openapi.json` on a deployment) is the reference; the bundled skill carries a condensed read-only copy in [`bundled/skills/aai-merkys/references/api-endpoints.md`](../../bundled/skills/aai-merkys/references/api-endpoints.md).

## Notes From Live Verification

- The OpenAPI document declares `HTTPBearer`, but user API keys are sent in the **`X-API-Key`** header. A bearer header is for service credentials, which are bound to one fixed service profile and cannot use the user routes. This CLI supports user API keys only.
- `POST /api/chat/{conversation_id}` returns only the answer text. The cited sources are on the stored assistant message, so `merkys ask` makes three calls: create a conversation, chat, then read the conversation's messages.
- Merkys can answer "I don't know" and still return sources. Callers should read the answer, not only check that sources exist.
- Deep research is polled, never streamed: `POST /v1/chat/deep-research` returns a `run_id`, and `GET /v1/chat/deep-research/runs/{id}` reports `pending`, `running`, `completed`, or `failed`.
- There is no default host. Each deployment has its own, so the profile requires `base_url`.
