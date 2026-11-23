# M6 K5 The notebook page (DEC-091 §1.8)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: the pocket's Notebook tab (`web/src/components/notebook/`): its own tabs (People, Places, Worries by neighbourhood, Policies, Leads), entries with their level in dots and words, source and date, history, readings' dates, leads with "Go there"; placeholders for public things not known yet with the lead that would reveal them (`NotebookView.gaps`, standing leads from the rules, `NotebookEntryView.lead`, `LeadView.subject`); people's strongest conviction (known) and whom they owe most (detailed); search; new entries first and marked, the view opening on their tab; `CommandDto::NotebookSeen` → `Command::NotebookSeen` (`KnowledgeLedger::mark_seen`, `Entry.seen`, saved, no format bump) so seeing clears the badge for good; the Today tab links to new entries; "Start with local knowledge" at setup (off by default); `e2e/notebook.spec.ts`
