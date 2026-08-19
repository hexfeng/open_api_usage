# Capability Matrix — 2026-08-19

| Provider | Source | Product authentication | Credential owner | Automated fields | Current validation |
|---|---|---|---|---|---|
| OpenAI API | Official API | Tested pasted Organization Admin Key with explicit elevated scope | Dashboard | Organization month-to-date costs, tokens, requests and daily cost buckets | Parser/error fixtures pass; user-confirmed live connection on 2026-08-18; exact official-dashboard reconciliation remains |
| DeepSeek API | Official API | Tested pasted API key with explicit account-balance scope | Dashboard | Availability, total, topped-up, granted balance | Parser/error fixtures pass; user-confirmed live connection on 2026-08-18; exact official-dashboard reconciliation remains |
| OpenRouter | Official API | Preferred localhost PKCE S256 normal key; manual normal-key fallback; separate advanced Management key | Dashboard after provider authorization | Key usage/limit or management credits/usage/derived remaining credits | Parser, S256, random-path binding and single-use callback tests pass; real PKCE browser acceptance remains |
| Google AI Studio API | Official API | Tested pasted Gemini API key with project/key scope | Dashboard | API-key validity through model listing and accessible `generateContent` model count | Parser/error fixtures pass; public key endpoint intentionally does not claim project spend, usage or prepaid balance |
| ChatGPT / Codex | CLI/OAuth | Confirmed shared session or official App Server browser/device-code login | Codex | Returned Codex quota windows, reset timestamps, optional credits and manual plan metadata | Parser, detected-identity UI, completion correlation, failed completion and no-logout boundaries pass; real browser/device-code account matrix remains |
| Google / Gemini CLI | CLI/OAuth · Experimental | Confirmed official CLI session with guided provider-owned login | Gemini CLI | Per-model remaining percentage, reset time and manual plan metadata | State detection and response fixtures pass; live success remains blocked because Gemini CLI is not installed/signed in on this PC |

## Important limits

- Codex values mean **Codex quota only**, never total ChatGPT Plus messages.
- Gemini values mean **Gemini CLI quota only**, never all Google AI Pro benefits.
- Subscription quota percentages and progress bars show **remaining quota**. Provider-native used percentages remain unchanged in SQLite and are converted only for display.
- Google AI Studio rate limits are project-level rather than API-key-level. The public API key connection does not invent balance or spend values.
- Gemini CLI reads its standard Windows Credential Manager entry first and supports the legacy `~/.gemini/oauth_creds.json` path; tokens stay in memory only, and refresh uses OAuth client configuration discovered from the installed official CLI package.
- A stale refresh keeps the last successful value.
- Cached last-success values and their source timestamps are restored before the first network refresh after launch.
- No exchange-rate conversion is performed.
- Remaining OpenRouter credits are explicitly calculated from official `total_credits - total_usage` fields.
- Demo values exist only in the browser prototype path and are not written to desktop storage.

Codex uses the stable local app-server account surface. Gemini CLI follows the current open-source CLI OAuth and `retrieveUserQuota` response contract, but remains Experimental because that internal service can change independently of this app.

See [Authentication and Connection Design](AUTHENTICATION.md) for credential ownership, browser callbacks, disconnect semantics and acceptance criteria.
