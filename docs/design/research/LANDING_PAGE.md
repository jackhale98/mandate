# Research: Landing Page

**Purpose:** Inform the public landing page and DEC-083 (landing page at the site root, the game at `play/`).
**Date:** 2026-11-18. One research pass over web sources (listed at the end). Most sources are marketing blogs and practitioner guides, not measured studies. Treat them as consensus, not data.

## 1. What makes a game page convert

1. **Show real gameplay early.** Gameplay converts better than cinematics or key art, and "many customers don't buy without seeing gameplay first". Steam's own rule is that screenshots must show the actual game. The first few images matter most. [GD-1, GD-2, IGJ, PK]
2. **One primary action, above the fold.** Most indie sites are one-pagers with no menu, built to send people to one button. Put the call to action high and repeat it at the end. Don't lead with navigation. [XS, EN]
3. **Answer three questions in the first screen:** what it is (genre in plain words), why it is different (the one or two mechanics nobody else has), and how to play now. Specific, concrete claims beat adjectives. [GD-1, IGJ]
4. **Captioned or annotated images do better than bare ones** (one source's claim, no data). Captions also make images honest: they say where each image came from. [SPA]
5. **Visual consistency:** the page, the screenshots and the share image should look like the same game. [PIO]

**For Mandate:** a free browser game has no store page in between, so the button goes straight to the game. "Play in your browser" is the action. The page must show the real map, the real town screen, and a real election result from the engine, each with a caption saying where it came from.

## 2. Tells of a generic AI-built or template site (to avoid)

Sources agree on a cluster of defaults. Each is fine on its own; together they read as "nobody chose this". [FI, VD, DEV, SD, 925, HM]

| Tell | What we do instead |
|---|---|
| Purple or indigo gradients; aurora glows; glowing text | The game's own palette (`--bg #11131a`, gold `--accent`), flat ink on paper; no gradients |
| Centred hero with a vague headline over a gradient | An asymmetric newspaper front page: masthead, a lead story with a concrete headline, a real map as the front-page picture |
| Three icon feature cards in a grid; everything in a card; nested cards | No feature cards. Each section has its own form: a results box, a ladder, a town directory, a ledger, a map strip, a notices box |
| Identical section rhythm (heading, subheading, grid, repeat) | Sections vary in width, density and shape, as columns on a front page do |
| Emoji as icons; meaningless status dots | No emoji. A handful of pixel-drawn marks (the logo, the ladder) in the game's style |
| Inter and other default fonts | Georgia for the paper, the system UI font for labels, as in the app; no font download |
| Generic fade-in on every element; particle backgrounds | Three small motions with a meaning: the ballot tally fills, the ladder's first rung lights, and nothing else moves. All off under reduced motion |
| Fake testimonials, logos, stats, ratings | None. The only numbers are the game's own (seed, vote shares), labelled as such |
| "Unlock", "elevate", "seamless", "empower", "next-level" | Plain verbs and specific mechanics: "you start on the town council", "polls can be wrong" |
| Stock illustrations | Screenshots and maps rendered by the real engine |
| Email capture, signup walls | None; there is no backend |

## 3. Performance and accessibility pitfalls

- **LCP:** on most pages the largest image is the LCP element. Give it `fetchpriority="high"`, never lazy-load it, keep it small, and serve AVIF or WebP. On phones, a text headline as the largest element in the first screen is even faster. [CWV, LH]
- **CLS:** images without `width` and `height` reserve no space. Late web fonts swap and shift text. Content inserted by script after load ("Continue your run") pushes everything down unless its space is reserved. Use system fonts and Georgia, reserve the space, set dimensions on every image. Good CLS is under 0.1. [CWV, DEVF]
- **Motion:** honour `prefers-reduced-motion`; motion should be small and tied to meaning (WCAG 2.3.3, AAA). [W3C-233]
- **Tap targets:** WCAG 2.2 AA requires 24×24 CSS px (2.5.8); 44×44 is the AAA size (2.5.5) and the app's own rule. Inline text links are exempt, but the page keeps every action link at least 44 px tall. [W3C-258, DQ]
- **Contrast:** 4.5:1 for body text and 3:1 for large text (WCAG 1.4.3). Gold on paper fails for small text, so the light theme uses a darker gold for text and buttons.
- **Without JavaScript:** the call to action is a plain link, and the page needs no script to read.

## 4. Service-worker migration (moving the app from `/` to `/play/`)

- vite-plugin-pwa's `selfDestroying` replaces a deployed worker with one that unregisters itself. It must keep the **same file name** at the same path, and the plugin's version deletes **every** cache on the origin, which would also wipe the new app's precache at `/play/`. [VPWA]
- A worker registered before a move can keep serving old caches to returning users until something replaces it. [VPWA-676]
- So we ship our own small `sw.js` at the old path: it installs at once, deletes only the caches made for its own scope, unregisters, and reloads the pages it controls.

## Sources

- [GD-1] Game Developer, "Best Practices: Game Websites and Landing Pages": https://www.gamedeveloper.com/game-platforms/best-practices-game-websites-and-landing-pages
- [GD-2] Game Developer, "Indie game marketing guide part 1: how to create landing pages": https://www.gamedeveloper.com/business/indie-game-marketing-guide-part-1-how-to-create-landing-pages
- [IGJ] Indie Game Joe, "Steam store page optimization: above the fold": https://indiegamejoe.com/steam-store-page-optimization-above-the-fold-best-practices
- [PK] presskit.gg, "Steam page optimization guide": https://presskit.gg/field-guides/steam-page-optimization-guide
- [XS] Xsolla, "Elements of a video game landing page": https://www.xsolla.com/blog/elements-of-a-video-game-landing-page
- [EN] Enjin, "How to create a website for your indie game": https://enjin.io/blog/how-to-create-a-website-for-your-indie-game-a-step-by-step-guide/
- [SPA] Steam Page Analyzer, "Store page mistakes": https://www.steampageanalyzer.com/blog/steam-store-page-mistakes
- [PIO] Playio, "Steam capsule art and wishlist conversion": https://blog.playio.co/steam-capsule-art-wishlist-conversion
- [FI] The Fountain Institute, "7 signs a UI has been vibe coded": https://www.thefountaininstitute.com/blog/signs-vibe-coded-ui
- [VD] Vandelay Design, "Why most AI-generated designs look the same": https://www.vandelaydesign.com/?p=61673
- [DEV] DEV Community, "Why every AI-built website looks the same": https://dev.to/alanwest/why-every-ai-built-website-looks-the-same-blame-tailwinds-indigo-500-3h2p
- [SD] Superdesign, "Why AI design looks generic": https://www.superdesign.dev/blog/why-ai-design-looks-generic
- [925] 925 Studios, "AI slop web design guide": https://www.925studios.co/blog/ai-slop-web-design-guide
- [HM] Hallmark anti-patterns catalogue: https://instagit.com/Nutlope/hallmark/what-are-hallmarks-anti-patterns-including-banned-fonts-gradients-and-layout-patterns/
- [CWV] Core Web Vitals checklist: https://www.corewebvitals.io/core-web-vitals/ultimate-checklist
- [LH] Unlighthouse, "LCP best practices": https://unlighthouse.dev/learn-lighthouse/lcp/best-practices-2026.md
- [DEVF] DEV Community, "Images, fonts, third-party scripts: LCP and CLS": https://dev.to/helloashish99/images-fonts-third-party-scripts-lcp-and-cls-idc
- [W3C-258] WCAG 2.2 SC 2.5.8 Target Size (Minimum), explained: https://tabnav.com/academy/wcag/success-criterion-2.5.8
- [DQ] Deque, "WCAG 2.2 updates": https://dequeuniversity.com/resources/wcag-2.2
- [W3C-233] WCAG 2.3.3 Animation from Interactions *(from prior knowledge; the search returned no source)*: https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html
- [VPWA] vite-plugin-pwa, "Unregister service worker": https://vite-pwa-org.netlify.app/guide/unregister-service-worker
- [VPWA-676] vite-plugin-pwa issue 676 (old worker keeps serving after a move): https://github.com/vite-pwa/vite-plugin-pwa/issues/676
