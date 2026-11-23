# Balance Log

Each tuning change records the parameter, old → new, the evidence (sweep or bot result), and the observed effect (TEST_STRATEGY §5).

Entries live in `docs/development/balance/`, one file per change, named `<date>-<short-slug>.md`; newest first by filename. Add one with `scripts/new-entry.sh balance "<title>"`.
