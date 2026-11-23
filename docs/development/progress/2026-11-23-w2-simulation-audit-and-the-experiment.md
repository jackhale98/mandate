# W2 simulation audit and the experiment runner (S2)

**Date:** 2026-11-23
**Status:** done (the audit and the runner); fixes not started

- The experiment runner (S2) in `crates/mandate-cli/src/lab/`: games from a seed and scenario under a driver (none, idle, a bot), exact forks, twins with and without an intervention (a law enacted at once, a content patch, a sector shock, a plant closure, a disaster, a recession), a metric snapshot (nation, towns, neighbourhoods, the player's ward) and a tally (races with the engine's factor breakdown, events with causes, crisis spans), a deterministic parallel map. `mandate-cli lab run|twin` drives it from the command line. No engine hook was needed.
- `mandate-cli audit policy|believe|sensitivity|diverge|leverage|stuck|readability|claims|gate`: the seven topics, the claims check and the nightly gate.
- The report: `docs/design/research/SIMULATION_AUDIT.md` (ten findings, a section per topic with commands, proposed fixes, the nightly gate).
- `.github/workflows/nightly.yml`: a non-blocking `audit gate` step in the `bots` job.
- Left: the fixes (record in the vote, rent bounds and private building, credit for results in approval, party turnover, honest ledgers), then making the gate blocking.
