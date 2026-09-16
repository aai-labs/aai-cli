# OAuth Token Refresh

How to provision durable delegated credentials for Microsoft Graph, Google Workspace, and Zoho REST services.

## Microsoft Graph

Microsoft delegated profiles use device authorization once and encrypted refresh tokens afterward:

```bash
aai-cli --config local/e2e.config.toml \
  --profile microsoft-e2e-delegated \
  microsoft auth login
```

The profile scope must contain `offline_access`. The CLI verifies the returned `/me` identity against `profile.user_id` when configured, then stores the refresh token under `refresh_token_secret`. Every later request exchanges that saved value noninteractively and persists Microsoft's rotated replacement token. Validate the durable session from a new process with `microsoft auth status`.

---

## Gmail

### One-time setup

1. Go to `https://console.cloud.google.com/` → create or select a project
2. Enable the **Gmail API**: APIs & Services → Enable APIs & Services → search "Gmail API" → Enable
   > If you skip this step the tool will return a `SERVICE_DISABLED` 403 error even with valid credentials.
3. Go to **APIs & Services → Credentials → Create Credentials → OAuth client ID**
4. Application type: **Desktop app** → give it a name → Create
   > Must be **Desktop app**, not **Web application**. Only a Desktop client may use a `http://localhost:<port>` redirect without registering it. A Web client rejects the flow below with `redirect_uri_mismatch` unless you add the exact redirect URI under **Authorized redirect URIs** first.
5. Note your `client_id` and `client_secret`

### Get a refresh token (one-time, lasts until revoked)

Run this URL in a browser (replace `CLIENT_ID`):

```
https://accounts.google.com/o/oauth2/v2/auth?client_id=CLIENT_ID&redirect_uri=http%3A%2F%2Flocalhost%3A8080&response_type=code&scope=https%3A%2F%2Fmail.google.com%2F&access_type=offline&prompt=consent
```

Paste it into the address bar as a single line — clicking a wrapped link in a terminal often sends only the first line, which Google rejects with `400: Required parameter is missing: response_type`.

Sign in → Allow → the browser lands on a "can't connect" page at `localhost:8080`. That is expected; nothing is listening. The code is in the address bar:

```
http://localhost:8080/?code=AUTH_CODE&scope=...
```

Copy the `code` value (URL-decode `%2F` back to `/` if the browser shows it encoded). It is single-use and expires in ~10 minutes.

Exchange it for tokens (replace `CLIENT_ID`, `CLIENT_SECRET`, `AUTH_CODE`):

```bash
curl -X POST https://oauth2.googleapis.com/token \
  -d client_id=CLIENT_ID \
  -d client_secret=CLIENT_SECRET \
  -d code=AUTH_CODE \
  -d redirect_uri=http://localhost:8080 \
  -d grant_type=authorization_code
```

Save the `refresh_token` from the response — you only see it once.

### Write credentials to the secrets file

```bash
aai-cli --config local/e2e.config.toml secrets set google.client_secret --value "CLIENT_SECRET"
aai-cli --config local/e2e.config.toml secrets set google.gmail_refresh_token --value "REFRESH_TOKEN"
```

### config.toml profile

```toml
[profiles.gmail-work]
provider = "google"
auth_type = "bearer_token"
client_id = "YOUR_CLIENT_ID.apps.googleusercontent.com"
client_secret_secret = "google.client_secret"
refresh_token_secret = "google.gmail_refresh_token"
user_id = "me"
```

---

## Zoho Mail REST

### One-time setup

1. Go to `https://api-console.zoho.com/`
2. Click **Self Client** → **Create Now** (if not already created)
3. Note the `client_id` and `client_secret` shown on the Self Client page

### Get a refresh token (one-time, does not expire)

1. On the Self Client page click **Generate Code**
2. Scope: `ZohoMail.accounts.READ,ZohoMail.messages.READ`
3. Time Duration: `10 minutes`
4. Click **Create** → copy the authorization code

Exchange it for tokens (replace `CLIENT_ID`, `CLIENT_SECRET`, `AUTH_CODE`):

```bash
curl -X POST "https://accounts.zoho.com/oauth/v2/token" \
  -d code=AUTH_CODE \
  -d grant_type=authorization_code \
  -d client_id=CLIENT_ID \
  -d client_secret=CLIENT_SECRET \
  -d redirect_uri=https://api-console.zoho.com/callback
```

Save the `refresh_token` from the response — it does not expire.

### Write credentials to the secrets file

```bash
aai-cli --config local/e2e.config.toml secrets set zoho.client_secret --value "CLIENT_SECRET"
aai-cli --config local/e2e.config.toml secrets set zoho.mail_refresh_token --value "REFRESH_TOKEN"
```

### config.toml profile

```toml
[profiles.zoho-mail-rest]
provider = "zoho"
auth_type = "zoho_oauth"
account_id = "YOUR_ACCOUNT_ID"
client_id = "YOUR_CLIENT_ID"
client_secret_secret = "zoho.client_secret"
refresh_token_secret = "zoho.mail_refresh_token"
```

To find your `account_id`, run once with a valid access token:

```bash
curl -H "Authorization: Zoho-oauthtoken ACCESS_TOKEN" \
  "https://mail.zoho.com/api/accounts"
```

---

## Google Sheets

### One-time setup

1. Go to `https://console.cloud.google.com/` → create or select a project
2. Enable both APIs: APIs & Services → Enable APIs & Services → search each name → Enable
   - **Google Sheets API** — required for all `sheets` commands
   - **Google Drive API** — required for `spreadsheets list`
   > If you skip this step the tool will return a `SERVICE_DISABLED` 403 error even with valid credentials.
3. Go to **APIs & Services → Credentials → Create Credentials → OAuth client ID**
4. Application type: **Desktop app** → give it a name → Create
   > Must be **Desktop app**, not **Web application** — see the note in the Gmail section above.
5. Note your `client_id` and `client_secret`

### Get a refresh token (one-time, lasts until revoked)

Run this URL in a browser (replace `CLIENT_ID`):

```
https://accounts.google.com/o/oauth2/v2/auth?client_id=CLIENT_ID&redirect_uri=http%3A%2F%2Flocalhost%3A8080&response_type=code&scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fspreadsheets%20https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fdrive.metadata.readonly&access_type=offline&prompt=consent
```

Paste it into the address bar as a single line — clicking a wrapped link in a terminal often sends only the first line, which Google rejects with `400: Required parameter is missing: response_type`.

Sign in → Allow → the browser lands on a "can't connect" page at `localhost:8080`. That is expected; nothing is listening. Copy the `code` value out of the address bar (`http://localhost:8080/?code=AUTH_CODE&scope=...`). It is single-use and expires in ~10 minutes.

Exchange it for tokens (replace `CLIENT_ID`, `CLIENT_SECRET`, `AUTH_CODE`):

```bash
curl -X POST https://oauth2.googleapis.com/token \
  -d client_id=CLIENT_ID \
  -d client_secret=CLIENT_SECRET \
  -d code=AUTH_CODE \
  -d redirect_uri=http://localhost:8080 \
  -d grant_type=authorization_code
```

Save the `refresh_token` from the response — you only see it once.

### Write credentials to the secrets file

```bash
aai-cli --config local/e2e.config.toml secrets set google.client_secret --value "CLIENT_SECRET"
aai-cli --config local/e2e.config.toml secrets set google.sheets_refresh_token --value "REFRESH_TOKEN"
```

If reusing an existing Gmail OAuth app, `google.client_secret` is already set — skip that line.

### config.toml profile

```toml
[profiles.google-sheets-work]
provider = "google"
auth_type = "bearer_token"
client_id = "YOUR_CLIENT_ID.apps.googleusercontent.com"
client_secret_secret = "google.client_secret"
refresh_token_secret = "google.sheets_refresh_token"
```

---

## How token refresh works in aai-cli

Every outgoing request automatically:

1. Reads `refresh_token`, `client_id`, and `client_secret` from the profile (via secrets file)
2. POSTs to the provider's token endpoint to obtain a fresh access token
3. Uses that access token for the actual API request

No manual refresh step or cron job is needed. If a stored access token (`token_secret`) is present instead of refresh credentials, it is used directly as a fallback.

---

## Quick reference

| | Gmail | Zoho Mail REST | Google Sheets |
|---|---|---|---|
| Refresh token TTL | Until revoked | Never expires | Until revoked |
| Access token TTL | ~1 hour (auto-refreshed) | ~1 hour (auto-refreshed) | ~1 hour (auto-refreshed) |
| `client_secret` secret key | `google.client_secret` | `zoho.client_secret` | `google.client_secret` |
| `refresh_token` secret key | `google.gmail_refresh_token` | `zoho.mail_refresh_token` | `google.sheets_refresh_token` |
| Profile name | `gmail-work` | `zoho-mail-rest` | `google-sheets-work` |
