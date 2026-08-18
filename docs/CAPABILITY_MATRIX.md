# Capability Matrix — 2026-08-17

| Provider | Source | Implemented | Automated fields | Current validation |
|---|---|---:|---|---|
| OpenAI API | Official API | Yes | Organization monthly costs, tokens, requests | Parser fixtures pass; live Admin Key comparison requires a user account |
| DeepSeek API | Official API | Yes | Availability, total, topped-up, granted balance | Parser fixtures pass; live dashboard comparison requires a user account |
| OpenRouter | Official API | Yes | Key usage/limit or management credits/usage | Both scopes have parser tests; live comparison requires a user account |
| ChatGPT / Codex | CLI/OAuth | Partial | Descriptor, scoped UI and manual plan metadata | Automatic quota fetch is not shipped until a stable Windows-safe connector is verified |
| Google AI / Gemini CLI | Browser session · Experimental | Partial | Descriptor, scoped UI and manual plan metadata | Automatic quota fetch is not shipped; explicit consent/domain isolation remains a release gate |

## Important limits

- Codex values mean **Codex quota only**, never total ChatGPT Plus messages.
- Gemini values mean **Gemini CLI quota only**, never all Google AI Pro benefits.
- A stale refresh keeps the last successful value.
- No exchange-rate conversion is performed.
- Demo values exist only in the browser prototype path and are not written to desktop storage.

CodexBar's current provider guide supports the descriptor/fetch-strategy separation used here. Its 2026 provider notes also indicate that Google individual Gemini CLI quota access changed, so this app keeps the connector experimental rather than silently scraping credentials.
