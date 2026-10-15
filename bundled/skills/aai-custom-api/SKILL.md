---
name: aai-custom-api
description: Call a configured Custom API connection with aai-cli, using absolute URLs within its configured origin and base path.
---

# Custom API connections

Select the assigned profile explicitly. API-specific endpoints, payloads and pagination come from assigned API documentation or skills; never infer them from a connection name.

`aai-cli --profile CONNECTION custom-api request --url https://api.example.com/v1/items --method GET --query limit=10`

Use `--json FILE` (or `--json -` for stdin) for a JSON body, and repeat `--header Name=value` for ordinary request headers. Configured headers cannot be overridden. Do not place credentials in command flags or read credential variables.

For a complete request object use `aai-cli --profile CONNECTION custom-api request --request-json -` with stdin:

```json
{"method":"GET","url":"https://api.example.com/v1/items","query":{"limit":10},"body":null,"headers":[]}
```

Only absolute public HTTPS targets within the profile origin and base path are accepted; redirects and ambiguous encoded paths are refused. Direct and isolated profiles use the same commands. Results are `{ "status": 200, "body": ... }`; body preserves JSON or text. Inspect status for upstream failures. There is no automatic pagination: follow the API's documented continuation using another in-scope URL. CLI validation and gateway failures use structured errors on stderr. Do not retry authentication failures without fixing the profile.
