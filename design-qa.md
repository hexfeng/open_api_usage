# Design QA

**Source visual truth**

- Dark list: `D:\Projects\AI_usage_dashboard\design\reference-dark-list.png`
- Light cards: `D:\Projects\AI_usage_dashboard\design\reference-light-cards.png`

**Implementation evidence**

- Dark list: `D:\Projects\AI_usage_dashboard\design\implementation-dark-list.png`
- Light cards: `D:\Projects\AI_usage_dashboard\design\implementation-light-cards.png`
- Minimum desktop: `D:\Projects\AI_usage_dashboard\design\implementation-960x700.png`
- Normalized comparisons: `D:\Projects\AI_usage_dashboard\design\compare-dark-list-1440.png` and `D:\Projects\AI_usage_dashboard\design\compare-light-cards-1440.png`

**Capture normalization**

- CSS viewport: 1440 × 1024, device-scale behavior controlled by the in-app browser.
- Raw source pixels: dark 1491 × 1055; light 1487 × 1058.
- Raw implementation pixels: 1425 × 1013 after browser scrollbar/chrome exclusion.
- Both sides were normalized with Lanczos resampling to 1440 × 1024 and placed together in one 2880 × 1024 comparison image. No browser chrome or device frame was included.
- States: dark + list and light + cards with identical 2026-08-17 data.

**Findings**

- No actionable P0, P1 or P2 findings remain.
- [P3] The theme control is kept in the persistent top bar instead of beside the page timestamp. This is an intentional response to the confirmed requirement that dark/light be global across Dashboard, Accounts and Settings.
- [P3] The OpenAI mark uses the closest neutral icon-library glyph because the installed brand library no longer distributes the OpenAI brand mark. DeepSeek, OpenRouter and Gemini use library brand assets; no handcrafted SVG or CSS illustration was introduced.

**Required fidelity surfaces**

- Fonts and typography: Manrope display text plus DM Sans/Segoe UI body text reproduce the source hierarchy; dense metric labels remain legible with no clipping or unintended wrapping.
- Spacing and layout: the final two-column composition, left summary/API stack, right subscription rail, large panel proportions, compact dividers and card rhythm match the source intent. The 960 × 700 check has no horizontal overflow.
- Colors and tokens: graphite/off-white themes, restrained blue/green/amber/violet status colors, borders and low-elevation surfaces remain consistent in all four theme/layout combinations.
- Image quality and assets: provider marks remain sharp vector assets from icon libraries. There are no raster placeholders, emoji, CSS art or generated decorative filler.
- Copy and content: account scope, source, freshness, reset window and Manual labels are all explicit. Codex and Gemini copy names the limited product scope.
- Accessibility: semantic buttons/links, pressed states, form labels, disabled state, focus-visible rings and reduced-motion handling are present. Browser checks found no clipped persistent controls at the desktop minimum.

**Focused-region evidence**

The normalized full-view images retain readable provider rows, metric labels, status dots, quota bars and footer metadata at 1:1 review size, so an extra crop was not needed. The account dialog, page navigation and account-detail drawer were inspected separately through live interaction.

**Comparison history**

1. First comparison found a P2 composition mismatch: the implementation placed the summary across the entire viewport and pushed Subscriptions below it, while the source used a left main column and a full-height right rail.
2. Fix: moved the summary and API panel into the left column; aligned the subscription rail to the summary top; added matching outer panels and vertical proportions; reduced inter-column spacing.
3. Post-fix evidence: both normalized comparison images show the same major-region order and proportions. No provider or subscription is duplicated, clipped or moved out of its intended group.

**Primary interactions tested**

- Dashboard / Accounts / Settings navigation.
- Dark/light theme toggle and list/card layout toggle with pressed state.
- Add-account dialog, disabled Save before a successful connection test, and close action.
- Provider detail drawer open/close.
- 960 × 700 minimum desktop layout and horizontal overflow check.
- Browser console errors checked: none.

**Implementation Checklist**

- [x] Match the confirmed two-column composition.
- [x] Verify both selected visual states at normalized 1440 × 1024.
- [x] Verify core navigation, toggles, dialog and detail interaction.
- [x] Verify minimum desktop width without horizontal overflow.
- [x] Check console errors.

**Follow-up Polish**

- If a licensed OpenAI brand asset is supplied later, replace the neutral atom glyph without changing card geometry.

final result: passed
