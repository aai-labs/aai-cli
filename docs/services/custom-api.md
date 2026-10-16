# Custom API connections

A named `provider = "custom_api"` profile calls absolute URLs on a configured public HTTPS origin and base path. It does not infer API endpoints or pagination. Provider-specific `request` commands retain their existing relative-path behavior.

```toml
[profiles.example]
provider = "custom_api"
base_url = "https://api.example.com/v1"
auth_type = "bearer_token"
token_secret = "example.token"

[[profiles.example.headers]]
name = "X-Workspace"
value = "workspace-name"

[[profiles.example.headers]]
name = "X-Additional-Key"
value_secret = "example.additional-key"
secret = true
```

Authentication options are `none` (default), `bearer_token` (`token`), `api_key_header` (`api_token`, required `api_key_header`), and `basic` (`username`, `password`). Each credential supports `_env` and `_secret` references; header values support `value`, `value_env`, and `value_secret`. Use the existing encrypted `secrets` commands to populate references. A named API key header may be `Authorization`, with the full value such as `Token <credential>` stored in the secret store. Custom API profiles never use ambient generic/provider token fallbacks. Explicit literal values take precedence over environment references, followed by encrypted secrets.

`config profiles set` and `config profiles validate` require `base_url` (absolute HTTPS, no query, fragment or credentials), default `auth_type` to `none`, and require the matching `*_secret` reference for the chosen authentication (`api_key_header` also needs the header name; `basic` accepts `username` or `username_secret`). Through these commands, header values are either a non-secret literal `value` or a `value_secret`; `value_env` and literal values marked `secret = true` are refused.

```sh
aai-cli --profile example custom-api request --method GET --url https://api.example.com/v1/items --query limit=10
aai-cli --profile example custom-api request --method POST --url https://api.example.com/v1/items --json request.json
```

Repeat `--query name=value` and `--header Name=value` as needed. Duplicate query names in flags retain the last value; use an array in the complete request object for repeated values. Use `--request-json FILE` or `--request-json -` for a complete request object (exclusive with individual flags):

```json
{"method":"GET","url":"https://api.example.com/v1/items","query":{"limit":10},"body":null,"headers":[{"name":"Accept","value":"application/json"}]}
```

Methods: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. Bodies may be JSON values or raw text strings in the complete request object. Results are `{ "status": 200, "body": ... }`, including upstream error statuses; body preserves JSON or text and known configured secrets are redacted. This command does not add pagination metadata. Validation, transport, and gateway errors are structured JSON on stderr with nonzero exit codes and sanitized messages.

Scope checks compare scheme, hostname, effective port and decoded path segment boundaries before credentials are attached. Request URLs must be absolute public HTTPS URLs without embedded credentials or fragments. Dot segments, duplicate slashes, encoded separators, nested escapes and control characters are refused. Redirect responses (301, 302, 303, 307, 308) are refused, never followed; other 3xx statuses such as `304 Not Modified` are returned in the result envelope. DNS resolution rejects private, internal and special addresses and pins the accepted addresses for the request; HTTP proxies from the environment are disabled. Request and response bodies are limited to 1 MiB, and one 30 second deadline covers DNS resolution, sending and reading the response. Configured headers cannot be overridden, and hop-by-hop, host and authorization request headers are reserved.

## Isolated connections

The same command forwards the complete request JSON to a trusted configured gateway. Only the gateway holds upstream credentials:

```toml
[profiles.isolated]
provider = "custom_api"
base_url = "https://api.example.com/v1"
auth_type = "none"
gateway_url = "http://credential-gateway:8000/gateway/v1/custom-api/request"
gateway_token_secret = "example.gateway-token"
```

`gateway_token` supports `_env` and `_secret` references. Gateway URLs may use HTTP for internal deployments or HTTPS. A gateway profile cannot contain upstream tokens, passwords, authentication, or configured headers, and a gateway token without a gateway URL fails closed. `config profiles validate` applies the same rules and requires `gateway_token_secret`. The gateway receives a bearer token and `{method,url,query,body,headers}`; it must enforce scope and protect credentials, returning `{status,body}`. Gateway failures never fall back to a direct request. An isolated profile's target URL still passes the same origin/path checks locally; the gateway enforces public destination DNS validation.
