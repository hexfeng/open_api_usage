# Capability Matrix — 2026-08-18

| Provider | Source | Implemented | Automated fields | Current validation |
|---|---|---:|---|---|
| OpenAI API | Official API | Yes | Organization month-to-date costs, tokens, requests and daily cost buckets | Parser fixtures pass; user-confirmed live connection and rendered fields on 2026-08-18; exact official-dashboard reconciliation remains |
| DeepSeek API | Official API | Yes | Availability, total, topped-up, granted balance | Parser fixtures pass; user-confirmed live connection and rendered fields on 2026-08-18; exact official-dashboard reconciliation remains |
| OpenRouter | Official API | Yes | Key usage/limit or management credits/usage/derived remaining credits | Both scopes have parser tests; user-confirmed live connection and rendered fields on 2026-08-18; exact official-dashboard reconciliation remains |
| Google AI Studio API | Official API | Yes | API-key validity through model listing and accessible `generateContent` model count | Parser fixture passes; public key endpoint intentionally does not claim project spend, usage or prepaid balance |
| ChatGPT / Codex | CLI/OAuth | Yes | Returned Codex quota windows, reset timestamps, optional credits and manual plan metadata | Parser fixtures pass; live Windows app-server read passed against the locally signed-in ChatGPT Plus account on 2026-08-18 |
| Google / Gemini CLI | CLI/OAuth · Experimental | Yes | Per-model used percentage calculated from official remaining fraction, reset time and manual plan metadata | Official response fixture passes; local missing-credential behavior verified; live success remains blocked because Gemini CLI is not installed/signed in on this PC |

## Important limits

- Codex values mean **Codex quota only**, never total ChatGPT Plus messages.
- Gemini values mean **Gemini CLI quota only**, never all Google AI Pro benefits.
- Google AI Studio rate limits are project-level rather than API-key-level. The public API key connection does not invent balance or spend values.
- Gemini CLI reads its standard Windows Credential Manager entry first and supports the legacy `~/.gemini/oauth_creds.json` path; tokens stay in memory only, and refresh uses OAuth client configuration discovered from the installed official CLI package.
- A stale refresh keeps the last successful value.
- Cached last-success values and their source timestamps are restored before the first network refresh after launch.
- No exchange-rate conversion is performed.
- Remaining OpenRouter credits are explicitly calculated from official `total_credits - total_usage` fields.
- Demo values exist only in the browser prototype path and are not written to desktop storage.

Codex uses the stable local app-server account surface. Gemini CLI follows the current open-source CLI OAuth and `retrieveUserQuota` response contract, but remains Experimental because that internal service can change independently of this app.
