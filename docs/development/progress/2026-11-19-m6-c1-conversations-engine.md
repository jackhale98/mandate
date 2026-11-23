# M6 C1 Conversations engine (DEC-093)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: `PlayerCommand::Talk` (`mandate_sim::talk`: seven base topics and their mechanics through the lobbying, approval and favour paths; replies composed from opener, memory quote, substance and closer with trait voices; keyed repetition guard on the dialogue stream; asking again gives another line and nothing new; half a slot per conversation), the shared `SituationCondition` extended with slot-relative variants and Fluent argument providers, speaker knowledge and discretion derived by rule (`characters::knows`), content types and validation (`mandate-content/src/conversation.rs`, `content/base/conversations/`, `talk.ftl`; the full line set is C3), `PlayerState.talk`, save format v17; no protocol views yet (C2)
