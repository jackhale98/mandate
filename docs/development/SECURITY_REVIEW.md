# Security review (M6 note 15)

Done 2026-11-23. Scope: the repository, its history, the built site, CI, and the browser-side import and storage paths. Not a penetration test. The game has no backend, no accounts and no remote requests, which removes most of the usual surface; what is left is the supply chain, the build pipeline, what a hostile file or URL can do inside the page, and the browser storage.

## Findings

| # | Item | Severity | Status |
|---|---|---|---|
| 1 | Secrets in tree and history (513 commits, all branches) | none found | clean |
| 2 | Secrets or remote origins in the built bundle | none found | clean |
| 3 | `npm audit` (web): 4 moderate, all in `vite-plugin-pwa` > `workbox-build` > `glob`/`minimatch`, build-time only; 0 in production deps | low in practice | open (no non-breaking fix; revisit on the next `vite-plugin-pwa` release) |
| 4 | `npm audit` (`tools/pixelkit`): no dependencies, no lockfile | n/a | nothing to audit |
| 5 | `cargo audit` (cargo-audit 0.23.0, RustSec database of 1562 advisories, 115 crates in Cargo.lock) | none found | clean |
| 6 | CI actions referenced by mutable tag (`@v4`) | medium (supply chain) | fixed: pinned to commit shas |
| 7 | CI token permissions defaulted to the repository setting | low | fixed: `contents: read` at top; deploy job adds only `pages`/`id-token` |
| 8 | No dependency audit in CI | low | fixed: non-blocking `audit` job |
| 9 | No Content Security Policy | medium | fixed: meta CSP on app and landing pages |
| 10 | Service worker `autoUpdate`: the "new version" prompt (`UpdatePrompt`) was dead code, the page reloaded itself under the player, and an open app never looked for updates | medium (stale builds, mid-turn reload) | fixed: `prompt` mode, hourly and on-focus update checks |
| 11 | Save import validated hostile bytes in the app's live engine; a panic or hang would poison the running game | medium | fixed: throwaway engine, 30 s timeout |
| 12 | Save import: no cap on header size or declared save size; text fields unbounded; string input had no cap | low | fixed |
| 13 | Inline `style` attribute in the icon sprite (blocked by the new CSP) | low | fixed (class + stylesheet rule) |
| 14 | `mandate-save` inflates up to 256 MiB (`MAX_PAYLOAD_BYTES`) | low-medium | open: engine crate, see "Follow-ups" |
| 15 | Source maps are published for the game (`sourcemap: true`) | info | accepted: the source is public |
| 16 | `spike-hub/` is a throwaway page published unlinked, built separately, with no CSP | info | open: remove before release (DEC-050) |
| 17 | The CSP is a `<meta>` tag: `frame-ancestors` and reporting cannot be set, so the game can be framed by another site (clickjacking) | low | open: needs response headers (custom domain on Cloudflare, DEC-104) |

## 1. Secrets

- History: `git log --all -p` searched for `AKIA`, `ghp_`, `gho_`, `github_pat_`, `sk-`, `xox[bp]-`, `AIza`, `-----BEGIN`, and `api_key`/`secret`/`token`/`password`/`bearer` assigned a quoted value of 8+ characters. No match in added lines. No `.env`, `.pem`, `.key`, `id_rsa` or `credentials` file was ever added.
- Tree: same patterns, no match. The only token use is `GH_TOKEN: ${{ github.token }}` in the `verified` job (the job's own short-lived token).
- Bundle (`web/dist`, with source maps): no key patterns. The words "secret" and "token" appear only as game content (a topic called `secret`). The only absolute URLs are the SVG/XML namespaces, React's error-page link, `github.com/jackhale98/mandate`, and the landing page's canonical and Open Graph URLs.
- Re-run before release with a scanner (gitleaks or trufflehog) in CI; the checks above were pattern greps.

## 2. Dependencies

- Web: see rows 3 and 4. The shipped dependencies (React, ReactDOM, Zustand, Dexie, Fluent, Workbox runtime) have 0 advisories.
- Rust: `cargo audit` ran in this session (the advisory database was reachable): no vulnerabilities and no warnings. The new CI `audit` job repeats it on every push.
- Licences are covered in `THIRD_PARTY_NOTICES.md`.

## 3. CI

- Every `uses:` is pinned to a commit sha with the tag in a comment. Pins: checkout 4.3.1, cache 4.3.0, upload-artifact 4.6.2, download-artifact 4.3.0, setup-node 4.4.0, upload-pages-artifact 3.0.1, deploy-pages 4.0.5, rust-cache 2.9.3, taiki-e/install-action 2.79.15. Recommendation: add `.github/dependabot.yml` with the `github-actions` ecosystem so pins are bumped by pull request (outside this task's `OWNS`).
- Workflows use `pull_request`, never `pull_request_target`, and reference no repository secrets, so forks run with a read-only token and nothing to steal. Third-party actions run with the job's token only.
- `audit` job: runs `npm audit --omit=dev --audit-level=high`, `npm audit` (report), and `cargo audit`; `continue-on-error: true`. Make it blocking (remove that line) before the first public or store release, once it is clean or each accepted advisory is recorded with a reason. It is not in `deploy`'s `needs`.
- The `verified` fast path and every other job's behaviour are unchanged.
- Repository settings to check by hand (not visible from code): branch protection on `main`, "Require approval for first-time contributors' workflows", and Actions default token permission set to read-only.

## 4. Content security policy

The pages have a `<meta http-equiv="Content-Security-Policy">`, added at build by `web/csp.ts` (never in `vite dev`, which needs inline scripts for hot reload).

Game (`play/`): `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; worker-src 'self'; manifest-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'`.

Landing and privacy pages: the same plus `style-src-attr 'unsafe-inline'` (the front page's bar chart sets a CSS variable per bar in a style attribute), without `wasm-unsafe-eval`, `form-action 'none'`, and the landing page's one inline script is allowed by its sha-256 (computed at build, so it cannot drift).

What was found, not guessed:
- No remote origin is needed. The code makes no `fetch`, WebSocket or beacon call; the engine `.wasm` is loaded from the site itself (`connect-src 'self'`). No web fonts (system fonts only) and no remote images.
- `'wasm-unsafe-eval'` is needed to compile WebAssembly. It does not permit `eval` or `new Function` (none in the bundle).
- Workers (`engine.worker`, `town.worker`) are same-origin module workers; no `blob:` workers.
- `data:` images: kept for the icon and inlined small assets.
- `style-src 'self'` with no `'unsafe-inline'`: the only inline style attribute (the icon sprite) was changed to a class. React and the renderers set styles through the CSSOM, which the policy allows.
- Limits of a meta policy: it governs the document only (workers load under their own, absent, policy); it cannot set `frame-ancestors`, `report-uri` or `sandbox`.
- Check: `e2e/csp.ts` is a Playwright `test` that fails any test on a CSP violation (`securitypolicyviolation` event or Chrome's "Refused to..." console message). `smoke`, `main-screen`, `run-transfer` and `landing` specs use it. Use it in new specs that touch new features (a new inline style or a new origin will fail there first).

## 5. Save import

Path: `RunTransfer.tsx` (file picker, drag and drop) > `persistence/runFile.ts` `decodeRunFile` > engine validation > `IndexedDbStore.putRun`.

Already in place: a 64 MiB file cap checked on `file.size` before reading; magic line, strict UTF-8, JSON header with a shape check (`checkHeader`: types, run id must be hex, mode and status enums), format version ("too-new" before anything else), declared length and CRC-32 of the save, engine integrity hash and version/migration checks, header-matches-save check, no rewinding an Ironman run; all failures are `RunImportError`s shown as a message (Fluent `ui-import-error-*`), nothing is stored until all pass. Nothing from a file is rendered as HTML: names and ids are React text; `innerHTML` is used in one place, with constant strings.

Added: header cap (16 KiB); declared save cap (40 MiB) checked before any decoding; base64 length must equal what the header declares (checked before decoding, so no allocation of a lying size); text fields capped at 200 characters; the size cap now covers string input; validation runs in a throwaway engine (`validateInProbe`) which is terminated afterwards, with a 30 s timeout, so a panic, an out-of-memory or a hang costs one disposable worker and not the running game (previously the shared engine loaded the bytes, and a panic left it unusable until reload). Tests: `runFile.hostile.test.ts` (oversized, lying header, truncated at every 7th byte, random binary, UTF-16, nested JSON bomb, hostile names, probe timeout and terminate).

Decoder guard for a follow-up (`crates/mandate-save/src/lib.rs`, not edited here): `decode_with_log` inflates with `MAX_PAYLOAD_BYTES = 256 MiB`. A valid deflate stream of that size fits in a 40 MiB file only at about 6:1, and zeros compress about 1000:1, so a ~260 KB file can make the engine allocate 256 MiB for the inflate plus `serde_json::Value` (several times that) in WASM, which can exhaust a phone's memory. Suggest: lower the cap to a measured multiple of the largest real save (measure the largest real save inflated, then set the cap at a few times that), check `payload_hash` (already done after inflate) and add an inflated-to-compressed ratio guard; and bound JSON depth and collection sizes during `from_value` (serde_json already limits recursion at 128). Also confirm `Engine::from_state` has no panicking path for a state with valid hash but inconsistent content (a hostile author can compute the hash); a fuzz test over mutated saves would settle this. The web side now survives such a panic.

## 6. Service worker and storage

- Scope: the game's worker is `play/sw.js`, scope `play/` (manifest `scope` and `start_url` the same). It precaches the build's own files (25 entries, about 9.5 MB: js, css, html, wasm, png, svg) and has no runtime caching rules, so nothing cross-origin is ever cached. Navigations fall back to `play/index.html`; `/spike-` paths are denied.
- `web/landing/public/sw.js` (the landing root) is a retired worker that only removes the pre-DEC-083 caches and unregisters itself; nothing registers it.
- Updates (the stale build the owner saw): the cause was `registerType: "autoUpdate"`: the new worker skipped waiting, took over and reloaded the page by itself, and the check for a new worker happens only when a page loads, so an installed app left open or resumed from the background stayed on the old build for as long as it lived; `UpdatePrompt` never showed because `onNeedRefresh` is not called in that mode. Now: `registerType: "prompt"`, `clientsClaim: true` (first install controls at once so offline works after one visit). A new build waits, the banner "A new version is ready. Reload" appears (existing component and Fluent keys), the player taps Reload, the worker activates and the page reloads. The app asks for an update every hour and whenever it returns to the foreground. Players on the old `autoUpdate` worker get one automatic reload when they receive this build; after that, the prompt. Not covered by an automated test (a service worker update needs two builds); verify by hand: deploy twice, keep the app open, expect the banner.
- Stored in the browser (also in the privacy page):
  - IndexedDB `mandate`: `runs` (saves: bytes plus metadata), `checkpoints` (Story-mode), `settings` (a random `device_id`).
  - IndexedDB `mandate-town-plans`: cached street layouts, newest 8.
  - localStorage: `mandate.mapLayer`, `mandate.pocketTab`, `mandate.pocketDocked`, `mandate.mapScope` (display choices).
  - Cache Storage: Workbox precache of the game's files, plus the worker registration.
  - No cookies, no sessionStorage. The landing page reads the `mandate` database's run list (read-only, never creates it).

## 7. XSS surface

- `dangerouslySetInnerHTML`, `eval`, `new Function`, `document.write`: none in `web/src` (tests excluded) or the built bundle.
- `innerHTML`: `hub/hubView.ts` sets a constant SVG string chosen from a closed set of marker names; `TownStreet.tsx` inserts the icon sprite built from constant glyph tables. No player text reaches either.
- Fluent: arguments go through `@fluent/bundle` into React text nodes (escaped by React); no argument is rendered as markup. Player-typed text (the seed field accepts digits only; town names come from the generator; the bug-report description is written into a JSON file) is never inserted as HTML. A hostile `.mandate` file's town name is shown as text and slugged for the file name.
- Open redirects and links: the two new links use a fixed relative URL and `rel="noopener"`.

## Follow-ups (owner or other tasks)

1. `mandate-save` decode guard, as in section 5.
2. `.github/dependabot.yml` for Actions, npm and cargo.
3. Make the `audit` job blocking before the first public release; add a secret scanner (gitleaks) to CI.
4. Remove `spike-hub` before release.
5. Headers (`frame-ancestors`, HSTS, `Permissions-Policy`) arrive with the custom domain on Cloudflare (DEC-104).
6. A manual deploy-twice check of the update banner.
7. A "delete all my data" button (also for privacy; see `LEGAL_GROUNDWORK.md`).
