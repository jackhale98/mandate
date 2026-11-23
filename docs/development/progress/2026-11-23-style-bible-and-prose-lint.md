# Style bible and prose lint

**Date:** 2026-11-23
**Status:** done (bible and samples await the owner's approval)

- `docs/design/STYLE_BIBLE.md`: the voice (four tonal anchors), rules of the sentence, register by text kind, 13 voice sheets, writing over variables, sensitive ground, twelve machine-writing habits with before and after, and the process (draft, lint, reviewer, owner's blind sample).
- `docs/design/STYLE_SAMPLES.md`: 20 passages for approval; five rewrite current lines (`talk-about-person-ambitious`, `ask-attend-meeting-kept`, `scene-disaster-helper`, `dilemma-plant-layoffs-broker-talks-after`, `talk-quote-stood-with-me`).
- Doctor: `prose-habit` (info; warn with `--strict-prose`) in `crates/mandate-cli/src/doctor/text.rs`, over every variant of every English message: stock phrases (`STOCK_PHRASES`), contrast frames, two lists of three in one message, more than one dash, an exclamation outside speech, length by kind.
- Base pack: 17 findings, all stock phrases (16, mostly the "will not forget it" / "will remember" closer) and one exclamation (`event-player-elected-mayor`). No contrast frames, paired lists, dash runs or over-long lines.

Left: no game text was rewritten (other agents own those files). The sample rewrites and the 17 findings are for a content pass after approval.
