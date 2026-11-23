# Security pass and legal groundwork (M6 notes 14 and 15)

**Date:** 2026-11-23
**Status:** done, pending the owner's decisions.

Security: findings in `docs/development/SECURITY_REVIEW.md`. Fixed: CI actions pinned and token permissions narrowed, non-blocking `audit` job, meta CSP on the app, landing and privacy pages (with `e2e/csp.ts` guard), service worker switched to prompt mode with hourly and on-focus update checks (the update banner now works), save import hardened (size caps, throwaway validation engine with timeout, hostile-file tests). Open: `mandate-save` inflate cap (engine crate), moderate build-time npm advisories, `spike-hub`, headers (custom domain).

Legal: `docs/design/LEGAL_GROUNDWORK.md` (not legal advice), `web/landing/privacy.html` (linked from the landing footer, title screen and pocket), `THIRD_PARTY_NOTICES.md`, fiction line (landing, privacy page, title screen via `app-fiction`). The AI-assistance statement is a draft in the groundwork doc, not live. Owner decisions listed at the end of that doc.
