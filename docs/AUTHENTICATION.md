# Authentication and Connection Design

Status: implemented engineering contract, updated 2026-08-19. Real-account acceptance items are listed in `DEVELOPMENT.md` and remain distinct from automated verification.

This document separates the connection behavior already implemented in the desktop MVP from the authentication behavior required before a wider product release. The dashboard does not have one universal sign-in model: API platforms, provider-managed OAuth flows, and local subscription sessions have different credential owners and security boundaries.

## 1. Goals

- Make the user explicitly choose which provider account is connected.
- Show the account identity, credential scope, data scope and source before completing a connection whenever the provider exposes them.
- Prefer provider-managed browser authorization when an official flow exists.
- Keep API keys, OAuth tokens and cookies out of SQLite, frontend state, analytics and logs.
- Preserve the last successful metrics when authentication expires, while changing the account state to `Authentication required` or `Stale` as appropriate.
- Keep `Remove from dashboard`, `Revoke credential` and `Sign out of provider` as distinct operations.

## 2. Non-goals

- A shared AI-account identity system or cloud account for this application.
- Password collection, browser-cookie extraction or form automation on provider login pages.
- Creating, rotating, deleting or revoking provider keys unless a future scope explicitly adds provider-side key management.
- Treating an API key as proof of ownership of an entire provider account when it only exposes key- or project-level data.
- Treating Codex quota as all ChatGPT usage, or Gemini CLI quota as all Google AI subscription usage.

## 3. Authentication modes

| Mode | Credential owner | User ceremony | Local storage | Providers |
|---|---|---|---|---|
| Pasted secret | User and provider | Paste a dedicated API/Admin/Management key, then test | Secret in Windows Credential Manager; opaque reference in SQLite | OpenAI API, DeepSeek, Google AI Studio, OpenRouter manual fallback |
| Provider browser authorization | Provider or provider runtime | Open the system browser, authorize, return through a loopback callback | Provider-managed session or returned key in Windows Credential Manager | ChatGPT/Codex, OpenRouter PKCE |
| Confirmed shared local session | Installed provider client | Detect account, display identity and scope, require explicit confirmation | No copied credential | ChatGPT/Codex current session reuse |
| Local CLI OAuth reuse | Installed provider CLI | Install and sign in to the provider CLI, then explicitly connect | Provider CLI owns the source credential; dashboard stores no raw token | Gemini CLI · Experimental |
| Manual metadata | User | Enter plan name, price and renewal date | Non-sensitive values in SQLite | ChatGPT/Codex and Gemini subscription metadata |

`Manual metadata` never describes quota collection. Automatic quota metrics retain their own `CLI/OAuth` source.

## 4. Common connection state machine

```text
Disconnected
    -> Detecting local/provider capability
    -> Consent required
    -> Authorizing or validating
    -> Connected
    -> Refreshing
    -> Connected

Authorizing -> Cancelled / Authentication required / Unavailable
Refreshing  -> Stale (last success retained)
Connected   -> Authentication required (credential expired, revoked or signed out)
```

Connection-dialog states such as `Detecting`, `Consent required`, `Authorizing` and `Cancelled` are implemented interaction states. Persisted account statuses remain `Live`, `Refreshing`, `Stale`, `Authentication required`, `Unavailable` and `Experimental`.

Every successful connection must record or derive:

```text
provider_id
authentication mode
credential owner
credential scope
metric scope
source
account identity label when available
observed time
last successful validation time
```

## 5. Provider authentication matrix

| Provider | Previous MVP | Implemented product behavior | Credential/data scope |
|---|---|---|---|
| OpenAI API Platform | User pastes an Organization Admin Key | Keep explicit Admin Key setup; link to the official Admin Keys page and explain elevated permissions | Organization Costs and Usage; not prepaid balance and not ChatGPT subscription |
| DeepSeek API Platform | User pastes an API key | Keep explicit API-key setup with test-before-save | Account balance returned to that key: availability, total, topped-up and granted balances |
| OpenRouter | User chooses normal API key or Management key and pastes it | Prefer official PKCE for a user-controlled normal key; retain manual Management key for account-level credits | Normal key is key-level; Management key is account/administrative scope |
| Google AI Studio API | User pastes a Gemini API key | Keep API-key setup for the current connector; treat any future Google Cloud OAuth billing integration as a separate capability | Key/project API access only; current public endpoint does not provide AI Studio usage, spend or prepaid balance |
| ChatGPT / Codex | Silently reuses the current local Codex session after the user presses Test | Detect and show the active account, require confirmation, then use App Server browser login or device code when signed out or switching accounts | Codex quota windows and optional Codex credits only |
| Google / Gemini CLI | Reads the existing Gemini CLI Google OAuth session | Add guided CLI detection and explicit account confirmation; keep Experimental until Google exposes a supported product integration for quota | Gemini CLI per-model quota only |

## 6. Detailed provider flows

### 6.1 OpenAI API Platform

Current implementation:

1. User creates an Organization Admin Key in OpenAI API Platform.
2. User pastes it into the desktop application.
3. The application calls Organization Costs and Completions Usage with Bearer authentication.
4. Only after validation succeeds is the key stored in Windows Credential Manager.
5. SQLite stores the generated `credential_ref`, account configuration and returned metrics.

Implemented product behavior:

- Label the action `Add Admin Key`, not `Sign in with OpenAI`.
- State that only Organization Owners can create Admin Keys and that the key has elevated organization permissions.
- Link directly to the official Admin Keys page.
- Display the scope as `Organization usage and costs` before saving.
- Do not imply that an ordinary project API key can read organization Costs or that an Admin Key represents account balance.
- Replacing a key must validate the new key before deleting the old local credential.
- Removing the account deletes the local credential reference; it does not revoke the Admin Key at OpenAI.

Official references: [Admin API Keys](https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/admin_api_keys), [Organization Usage and Costs](https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage).

### 6.2 DeepSeek API Platform

Current implementation:

1. User creates and pastes a DeepSeek API key.
2. The application calls `GET /user/balance` with Bearer authentication.
3. A successful response validates the key and returns account availability plus balance components.
4. The key is stored in Windows Credential Manager after successful validation.

Implemented product behavior:

- Label the action `Add API Key`; DeepSeek does not currently provide a product OAuth flow used by this connector.
- Explain that the returned balance is account-level data authorized by the key.
- Keep total, topped-up and granted balance separate.
- Never infer spend from a decrease in balance.
- Treat `401` as invalid/revoked authentication and `402` as an authenticated account with insufficient balance.
- Removing the account does not revoke the key at DeepSeek.

Official reference: [DeepSeek account balance](https://api-docs.deepseek.com/api/get-user-balance/).

### 6.3 OpenRouter

Current implementation supports three deliberately separate paths:

- Preferred provider browser authorization -> localhost PKCE S256 -> user-controlled normal API key.
- Normal API key -> `GET /api/v1/key` -> key usage, limit and remaining limit.
- Management key -> `GET /api/v1/credits` -> account total credits and total usage; remaining credits are calculated explicitly as `total_credits - total_usage`.

Implemented product behavior:

1. Make `Connect with OpenRouter` the preferred normal-key path.
2. Generate a high-entropy PKCE verifier and S256 challenge.
3. Bind a temporary loopback callback to `127.0.0.1` on a random free port.
4. Open OpenRouter `/auth` in the system browser.
5. Accept the code only on the callback path created for this connection attempt, then exchange the one-time code and verifier for a user-controlled API key.
6. Store the returned key in Windows Credential Manager, then validate it through `/api/v1/key`.
7. Keep `Add Management Key` as a separate advanced option for account-level credits. Do not silently upgrade a normal OAuth-created key to management scope.

PKCE requests must use S256, use a cryptographically random per-attempt callback path, expire before the provider's 10-minute authorization-code limit or on cancel, accept only one callback, and never put the verifier or returned key in logs. OpenRouter's documented flow does not currently define an OAuth `state` parameter, so the design must not claim to validate one. Localhost callbacks are explicitly supported for local-first applications.

Official references: [OpenRouter OAuth PKCE](https://openrouter.ai/docs/guides/overview/auth/oauth), [current API key](https://openrouter.ai/docs/api/api-reference/api-keys/get-current-key), [account credits](https://openrouter.ai/docs/api/api-reference/credits/get-credits).

### 6.4 Google AI Studio API

Current implementation:

1. User creates and pastes a Gemini API key from Google AI Studio.
2. The application calls the Gemini models endpoint using `x-goog-api-key`.
3. Success validates API access and counts accessible `generateContent` models.
4. The key is stored in Windows Credential Manager.

Implemented product behavior:

- Label the action `Add Gemini API Key`, not `Sign in with Google`.
- Display the project/key scope and state that API keys inherit project and billing-account settings.
- Do not display inferred usage, spend or prepaid balance from model-list access.
- Link to AI Studio Usage and Billing for unsupported financial information.
- A future Google Cloud OAuth billing connector would require its own consent, scopes, capability validation and account type; it must not be silently added to this API-key connection.

Official references: [Gemini API key setup](https://ai.google.dev/gemini-api/docs/get-started), [Gemini API billing and usage](https://ai.google.dev/gemini-api/docs/billing).

### 6.5 ChatGPT / Codex

Implemented product behavior:

1. Locate the installed Codex executable, preferring `CODEX_CLI_PATH`, then the ChatGPT/Codex Desktop bundle, then `PATH`.
2. Launch a new local `codex -s read-only -a untrusted app-server` child process and call `account/read` with `refreshToken: false`.
3. Display the returned email, plan and Codex-only scope. A detected account is not saved until the user selects `Use this account`.
4. For a missing or switched account, call official `account/login/start` in browser mode, correlate the `account/login/completed` notification by `loginId`, then re-read identity. Device-code mode is available as a fallback.
5. Cancel by calling `account/login/cancel`; timeout terminates the pending App Server. Neither dashboard removal nor authentication cancellation calls `account/logout`.
6. After consent, read only Codex quota windows, reset times and optional Codex credits. Store no Codex credential reference in SQLite or Windows Credential Manager.

Implemented flow:

```text
Connect Codex
    -> account/read
    -> existing account: show email, plan and Codex-only scope
         -> Use this account
         -> Sign in with another account
    -> no account / switch account
         -> account/login/start(type=chatgpt)
         -> open returned authUrl in the system browser
         -> wait for account/login/completed and account/updated
         -> account/read + account/rateLimits/read
         -> show verified identity and save the monitoring connection
```

Requirements:

- Use Codex App Server managed ChatGPT authentication; do not construct OpenAI OAuth URLs or read Codex token files.
- Prefer browser login. Offer `chatgptDeviceCode` when the loopback callback is blocked or brittle.
- Keep the App Server process alive for the complete pending-login lifecycle and support cancellation and timeout.
- Display the returned email and plan before final confirmation when available.
- `Remove from dashboard` deletes only the monitor configuration and cached metrics.
- `Sign out of Codex` is a separate high-impact action because Codex Desktop, CLI or IDE may share the cached session. It must never happen as a side effect of removing the dashboard account.
- The initial product should support one active system Codex identity unless isolated multi-account credential ownership is designed and verified.

Official references: [Codex App Server authentication](https://learn.chatgpt.com/docs/app-server), [Codex sign-in and credential storage](https://learn.chatgpt.com/docs/auth).

### 6.6 Google / Gemini CLI

Implemented data access:

1. Require the official Gemini CLI to be installed and signed in.
2. Read its standard `gemini-cli-oauth` / `main-account` Windows credential, with legacy `~/.gemini/oauth_creds.json` fallback.
3. Use the installed official CLI package's OAuth client configuration only when an expired access token must be refreshed in memory.
4. Call `loadCodeAssist` and `retrieveUserQuota` and parse per-model remaining fraction and reset time.
5. Store no raw Google OAuth credential in SQLite or application logs.

Implemented connection boundary:

- Detect `Gemini CLI not installed`, `Installed but signed out`, `Signed in` and `Credential invalid` separately.
- For an existing session, show the detected Google identity when the CLI exposes it and require explicit confirmation before monitoring.
- When signed out, guide the user to the official Gemini CLI `Sign in with Google` flow and poll only for completion; do not automate Google login forms.
- Do not copy Gemini CLI OAuth tokens into the dashboard's own credential namespace.
- Do not register or imitate a Google OAuth client merely to call the current internal quota service.
- Keep the connector `Experimental` until a stable supported interface exists or the local CLI integration passes compatibility testing across supported releases.
- Removing the dashboard account must not sign the user out of Gemini CLI.

Gemini CLI officially supports interactive `Sign in with Google` and exposes quota through `/stats model`; the dashboard's automated `retrieveUserQuota` integration is still an internal-contract dependency.

Official references: [Gemini CLI install and Google sign-in](https://github.com/google-gemini/gemini-cli/blob/main/docs/get-started/index.md), [Gemini CLI commands](https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/commands.md).

## 7. Credential storage and ownership

### Current persistent model

```text
SQLite AccountConnection
    credential_ref  -> opaque identifier only
    credential_kind -> admin | api_key | management | local_oauth

Windows Credential Manager
    credential_ref  -> secret value for pasted keys

Provider-owned local store
    Codex or Gemini CLI OAuth session
```

### Persisted product metadata

The connection model makes the following explicit:

```text
auth_mode            pasted_secret | provider_oauth | shared_local_session | local_cli_oauth
credential_owner     dashboard | codex | gemini_cli | provider
identity_label       optional email, workspace, project or key label
identity_fingerprint non-secret stable identifier when available
consented_at         local timestamp
last_validated_at    provider observation timestamp
```

Identity labels are personal data even when they are not credentials. Persist only what the UI needs and include them in local-data deletion.

## 8. Browser authorization requirements

- Open authorization URLs in the user's system browser, not an embedded WebView.
- Use a loopback listener bound only to `127.0.0.1` on an ephemeral port unless the provider's App Server owns the callback.
- Use PKCE S256 and a cryptographically random verifier when the provider supports PKCE.
- Validate a per-attempt state value only where the provider flow defines one.
- Associate callback path, verifier, optional state and provider login ID with one attempt; reject reuse and concurrent cross-account completion.
- Apply a short timeout, expose Cancel, close the listener and terminate the pending App Server on completion or failure.
- Never log authorization URLs containing codes, callback query strings, tokens, keys, verifiers or cookies.
- After callback success, re-read provider account identity and scope before marking the connection `Live`.

## 9. Error and recovery semantics

| Condition | UI state | Data behavior | Recovery |
|---|---|---|---|
| Invalid/revoked pasted key | Authentication required | Keep last success, do not overwrite with zero | Replace credential or remove account |
| OAuth cancelled or timed out | Cancelled for a new connection; Authentication required for reauthorization | Do not create a new saved account; keep an existing account's last success | Restart connection |
| Provider permission denied | Authentication required | Keep last success | Show required credential scope/role |
| Provider rate limited | Stale | Keep last success | Honor provider retry guidance and scheduler backoff |
| Network unavailable | Stale | Keep last success | Automatic retry |
| Local CLI missing | Unavailable | Keep saved configuration and cached data | Install/locate CLI |
| Local CLI signed out | Authentication required | Keep last success | Complete provider-owned login |
| Experimental contract changed | Experimental or Unavailable | Keep last success and diagnostics | Update adapter; never fabricate replacement metrics |

## 10. Disconnect, revoke and logout

- `Remove from dashboard`: delete account configuration, cached metrics/history and dashboard-owned credential. It does not revoke provider credentials or sign out shared local clients.
- `Replace credential`: validate and store the new credential first; update the reference atomically; then delete the previous local credential.
- `Revoke at provider`: open the official provider credential page. The MVP remains read-only and does not perform revocation.
- `Sign out of Codex/Gemini CLI`: a separate explicit action, disabled by default, with a warning about impact on other installed clients.

## 11. Acceptance criteria for product-grade authentication

- Each provider shows the exact authentication mode, credential owner and metric scope before connection.
- Existing Codex/Gemini sessions are never attached without an in-app confirmation that names the detected account when available.
- Codex browser login completes through App Server notifications and survives cancel, timeout, browser close and invalid-session cases.
- Codex device-code fallback is verified where enabled by the user or workspace.
- OpenRouter PKCE uses S256, a random loopback port and callback path, single-use code exchange and secure key storage without claiming unsupported `state` handling.
- OpenAI Admin, DeepSeek, OpenRouter Management and Gemini API keys are tested before save and can be replaced without losing the last working credential on failure.
- Removing a dashboard account does not log the user out of Codex or Gemini CLI and does not claim to revoke provider-side keys.
- SQLite, logs, crash reports and UI error messages contain no full API key, OAuth token, Cookie, authorization code or PKCE verifier.
- Invalid credentials, insufficient permissions, insufficient balance, rate limits, offline state and provider-source changes produce distinct diagnostics.
- Cached successful data remains visible as `Stale` after refresh failure and retains its original observation timestamp.
