# M6 Bug reports with replay (owner, 2026-11-19)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: "Report a problem" in the pocket, the error banner and the title screen (`web/src/components/report/`): one `mandate-report-<seed>-<date>.json` (format 1) with versions, build, seed and setup, the run log, the save, the state hash, recent caught errors, viewport/DPR/user agent and the tester's note; download, or the share sheet on phones; an accessible dialog that says what the file holds and that nothing is sent. The run log (`mandate_save::runlog`: new-game setup, every command, a state hash after the new game and each turn) is kept in the save beside the state: **save format v18** (`(17, run_log)` migration; renumber at merge if another bump lands first); `Engine::restore_log`; `reportCore`/`replayReport` on the WASM session. `mandate-cli replay <report> [--until N --out F] [--dump] [--wasm]` (`scripts/wasm-replay.mjs`) checks every checkpoint, the end and the bundled save, and names the first diverging turn and its commands; `e2e/report.spec.ts`; TEST_STRATEGY "Reproducing a tester's bug"
