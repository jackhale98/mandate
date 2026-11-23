# W1 play notes: labels, party line, ward view

**Date:** 2026-11-23
**Status:** done (owner notes 1, 8, 9)

- Note 1: one label rule, `characters::knows::role_label` (post, candidate, else trade; politicians keep no occupation in state, so it is derived and stable per character). "Out of office" only for the player. Used by person refs, sheets and chamber members. Test `role_labels`.
- Note 8: `politics::party_line` (platform against the bill's attitude axes, sponsor term, free band, reason). Player whip and NPC votes share it. `BillView.party_line_reason`. Weights in `politics.jsonc`. Tests `party_line`. Balance entry written.
- Note 9: `BillView` gains `ward_lean`, `ward_for_pct`, `ward_against_pct`, `ward_certainty`; `Chamber.tsx` renders them. Same coarseness as before.
- Left: `knows::role_key` (topic roles) unchanged on purpose: out-of-office politicians still have no topic role.
