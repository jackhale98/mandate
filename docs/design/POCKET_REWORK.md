# The pocket, reworked

**Status:** design for the owner (2026-11-25). Nothing here is built. No app code, content or test changed. A decision record is proposed in §9.

The owner: "The note book overall is very long so hard to review on a phone. Should we consider reworking and improving our overall menu ui?" This note says what the menu is for, counts what each job costs today, offers three structures, and recommends one.

**Mock-ups.** Sixteen phone screens (today's design and the three structures, four screens each) in one page, `pocket-mock/index.html`, with a PNG of each opening view. They are in the task's session folder, which is not kept: the task report gives the full path, and the lead puts the page where the owner can open it. `build.js` beside it makes the page and the PNGs again.

**In short.**
- Today's pocket is long for five reasons that can each be counted (§1.3). The largest is the person card: about 205 px each, so 60 people are 20.7 screens.
- All three structures bring every opening view under two screens. They differ in how much of the screen the head takes and in where reference lives.
- **Recommended: B, "Today and the Notebook".** One switch, one row of tabs, six people on the first screen where today shows one. C's map list follows when the map layers land.

---

## 1. What the player comes to the menu to do

### 1.1 How this was counted

- **The phone:** 393 × 793. The pocket is at most 85% of the screen high (`web/src/styles.css:431`), which is 674 px. **One screen** below means 674 px of the sheet.
- **Screens scrolled** is how far the sheet must scroll to bring the thing wholly into view.
- **Taps** start at the street with the pocket closed and left on its Today tab. The pocket reopens on the tab it was left on (`web/src/state/devicePrefs.ts`, `readPocketTab`), so add one tap when it was left elsewhere.
- **Today's lengths** come from a redrawing of the pocket from its code, with the app's own CSS and a mid-career turn (one decision, three asks and promises, two bills, 60 people known), measured in Chromium. It is not the running app: this task builds nothing. The lead's measurement of the real app at turn 4 is quoted beside it where they differ. The redrawing is the first row of the mock-ups.
- **How often** is a design judgement in bands. Nothing measures it: the game has no telemetry, and the lab's bots send commands without opening a menu. The M6 tester checkpoint is the place to check the bands.

### 1.2 The jobs, and today's cost

| # | Job | How often | Taps | Screens scrolled | Where in the code |
|---|---|---|---|---|---|
| 1 | Answer a decision | Several a session | 3: Pocket, Choose, the option | 0 | Decisions is the first card (`Pocket.tsx:178`). The decision sheet itself is outside this rework |
| 2 | See what is asked of me | Every turn | 1 | 0.2 to the first Accept; 1.3 to the end of three asks | The asks card under Decisions (`Pocket.tsx:179`); each ask is five to seven lines (`asks/Asks.tsx`, `AskItem`) |
| 3 | Find a person and go to them | A few times a session | 3: Pocket, Notebook, Go to them. 4 with the search box | 0.5 for the mayor; 15.6 for the 50th card. The list is 20.7 long | `notebook/People.tsx:112-183` (the card), `:216-241` (key figures, then everyone else) |
| 4 | Check what I know about a place or a worry | A few times a session | 3: Pocket, Notebook, Worries or Places | About 1 to 2 (not drawn: one card an entry, as People) | `notebook/Notebook.tsx:236-264`. The Town tab repeats each neighbourhood's top worry (`TownCard.tsx:127-162`) |
| 5 | See where an arc stands and what I can do in it | Once or twice a session while one runs | 2: Pocket, Story so far | 3.2, to the links under the Today tab | `StorySheet.tsx`: an arc is a list of dated lines. **Nothing says what the player can do** |
| 6 | Read a bill's impact | Around each vote | 3: Pocket, the bill's place button, "Why?" | 2.2 to "Your bills", then the place panel | `Pocket.tsx:46-69`, `BillCard.tsx`. **The impact estimate is not built** (`IMPACT_ESTIMATE.md` §6) |
| 7 | Check my standing and the next election | Once a session; more in a campaign | 1 | 2.7 to the election; 3.1 to the last track | The last things on Today (`Pocket.tsx:192-198`) |
| 8 | Save and leave | Once a session | 2: Pocket, Save & quit | 3.3 from Today; **19.6 from People** | The links sit under whichever tab is open (`Pocket.tsx:293-320`) |
| 9 | Go to a place without walking (DEC-051) | Several a turn in quick mode | 3: Pocket, Places, the place. 2 by the map | Up to 2.9 (the tab is 3.9 long: the lead's measurement) | `PlaceList.tsx`: tiles two to a row, at least 64 px (`styles.css:383-388`) |
| 10 | Take an agenda action | Up to three a turn in quick mode | 3 or more: Pocket, the action, confirm | 1.3 to the Agenda | `Pocket.tsx:102-145`: a help paragraph, eleven choices in a campaign (`ActionForm.tsx:213`), three buttons |

The redrawn Today tab is **4.4 screens** on this busy mid-career turn; the lead measured **2.9** at turn 4 with less waiting. The redrawn People list is **20.7 screens** with one whole person on the first screen and about two on each screen after.

### 1.3 Why it is long

1. **The person card.** Six to eight lines and two buttons: 194 to 237 px in the redrawing (`People.tsx:128-181`). Key figures are pinned first: the broker, the rival, the executive and every member of the legislature (`notebookModel.ts:253`). On a nine-seat council that is ten cards, 2.8 screens, before anyone else.
2. **Chrome.** In the notebook the first card starts 47% of the way down the sheet: the head, the pocket's tabs, a title, a note, the search box and a second row of tabs (`Notebook.tsx:269-314`).
3. **Everything is open.** The agenda shows its help text and every choice whether or not the player wants one. No list folds.
4. **Doing and reference share a page.** Today holds what needs an answer, the tools, the standing and the links, in one column.
5. **The links are at the bottom of every tab.** Eight buttons in three rows, under the whole scroll.

Four things the count turned up on the way:
- **Two tabs are called Places:** the pocket's (go there) and the notebook's (hidden places found). `ui-pocket-tab-places` and `ui-notebook-tab-places` both read "Places".
- **Town facts are in two places.** Who runs things and the rival's plans are on the Town tab (`TownCard.tsx:73-88`) and under People (`People.tsx:242-249`).
- **One query a card.** People asks the engine for a character sheet for every card after every command (`People.tsx:28-58`), and for a talk view on each "Go to them" (`People.tsx:84-92`). At 60 people that is 60 queries a command.
- **The bar can carry four badges** (`screens/Game.tsx:235-238`). Drawn with the app's CSS for 440 px and under (`styles.css:694-695`), four badges wrapped to three rows and overflowed the 44 px button; the mock-ups therefore show three. `gamebar.spec.ts` tests three. Not seen in the running app; worth one test.

### 1.4 What the tests hold today, and a rework must carry over

| Held by | What | Carried over as |
|---|---|---|
| `web/e2e/pocket.ts` (`openPocket` in 18 specs, `showPlaces` in 14, `openPeople` in 4) | The pocket opens on a named tab; Places lists every place; People shows the key figures | The helpers keep their names and arguments; only their insides change |
| `spot-<kind>`, `you-are-here` (15 specs) | Every place is a button reachable without walking (DEC-051) | The same ids on the new rows |
| `skip-to-places` (3 specs) | A skip link opens the place list and moves the keyboard into it | Kept, wherever the list lives |
| `decisions.spec.ts`, `asks.spec.ts` | A decision is always listed and answerable from the pocket (DEC-079); an ask can be accepted or declined there | Rows under "Needs you"; the same ids inside the opened row |
| `notebook.spec.ts` | The badge rises when something is learned and clears when seen; search finds an entry; a lead's Go walks there; local knowledge starts full | Kept. Its assertions on "Not known yet" and `data-level` change by design (§3.4) |
| `officeFigures` (4 specs), `talk-button`, `notebook-go-to` | Everyone in office is listed with Talk and Go to them | The "In office" section; the same ids |
| `gamebar.spec.ts` | The bar at 320 to 390 px: nothing cut, 44 px targets, no sideways scroll | Untouched by A and B. C adds a badge to the Map button and must pass it again |
| `open-story`, `open-term-review`, `open-legacy`, `report-problem`, Save & quit (11 specs) | The links | The same ids inside the More menu; one helper adds the extra tap |
| `PocketTabs.test.tsx`, `Notebook.test.tsx`, `notebookModel.test.ts`, `Asks.test.tsx` | Tab keys, grouping, search, ask text | Rewritten with the components |

---

## 2. Three structures

### A. Four tabs, shorter (conservative)

Today, Town, Places and Notebook stay where they are, and DEC-088's tabs stand. Each tab's contents become rows. Today leads with what needs an answer, then what is under way; the agenda and the standing fold to one row each; the links move to a "More" menu in the head. Town becomes fact rows with its neighbourhoods folded. Places becomes one line a place, the player's neighbourhood open and the rest folded. The notebook keeps its five tabs as a second row, in words without icons, and its search box.

### B. Today and the Notebook (splits doing from reference)

The pocket has two sides and one switch, which is also its title. **Today** is for doing: no tabs, one page. **The Notebook** is for looking things up: one row of four tabs, People, Places, Town and Policies. The pocket's Town and Places tabs fold into it. Places is the single place list: each neighbourhood is a row with its kind, ward and worries, opening to its places, each with "Go". Town holds the town card, the town's worries, the headline numbers, who runs things and the rival's plans, once. Search is a glass at the end of the tab row and searches the whole book.

### C. The pocket for doing, the map for knowing (splits doing from reference)

The bar already has two buttons, Map and Pocket. C makes them the two halves of the menu. The pocket is only Today, with a search field that answers "where is…". Everything the player knows moves onto the town map (DEC-086): layers for people, places, worries and wards, and one for each running arc, each with its list in a drawer under the map. The list is the text path for a screen reader, as the List view was. This takes "let the map carry browsing" as far as it goes.

### 2.1 Judged against the jobs

Taps, then screens scrolled. Measured on the mock-ups with the same content as §1. Jobs 4 and 9 are not drawn in A and B: they are reckoned from the row heights (57 px a row).

| # | Job | Today | A | B | C |
|---|---|---|---|---|---|
| 1 | Answer a decision | 3 · 0 | 3 · 0 | 3 · 0 | 3 · 0 |
| 2 | See what is asked of me | 1 · 0.2 to 1.3 | 1 · 0 | 1 · 0 | 1 · 0 |
| | …and answer an ask | 2 · 0.2 | **3** · 0 | **3** · 0 | **3** · 0 |
| 3 | Go to a person who matters now | 3 · 0.5 to 19 | 3 · 0 to 0.2 | 3 · 0 | 3 · 0 to 0.3 |
| | …any of the other 53 | 3 · up to 19, or 4 with search | 4 · under 1 | 4 · under 1 | 4, or 3 by their portrait |
| 4 | What I know about a place or worry | 3 · 1 to 2 | 3 · 0 | 3 · 0 | 2 · 0 |
| 5 | Where an arc stands, what I can do | 2 · 3.2, and no actions | 2 · 0 | 2 · 0 | 2 · 0 |
| 6 | A bill's impact | 3 · 2.2, and no estimate | 2 · 0.1 | 2 · 0 | 2 · 0.1 |
| 7 | Standing and the election | 1 · 2.7 to 3.1 | 1 · 0.4 | 1 · 0.4 | 1 · 0.4 |
| 8 | Save and leave | 2 · 3.3 (19.6 from People) | **3** · 0 | **3** · 0 | **3** · 0 |
| 9 | Go to a place without walking | 3 · up to 2.9 | 3 to 4 · under 1 | **4 to 5** · under 1 | 2 · 0 |
| 10 | Take an agenda action | 3 · 1.3 | **4** · 0.3 | **4** · 0.2 | **4** · 0.3 |

Bold is a tap more than today. Scrolling falls everywhere.

| Measure | Today | A | B | C |
|---|---|---|---|---|
| Opening view, screens | 4.4 (2.9 at turn 4) | 1.4 | 1.4 | 1.5 |
| People with 60 known, screens | 20.7 | 1.5 | 1.3 | 1.7 |
| Whole people on the first screen | 1 | 5 | 6 | 4 |
| First item starts, % down the sheet (opening view) | 20 | 20 | 13 | 21 |
| The same, on People | 47 | 37 | 20 | 63, the map counted |
| An arc in progress, screens | 2.0, no actions | 1.8 | 1.8 | 2.1 |

### 2.2 What gets worse

| | What gets worse |
|---|---|
| **All three** | Answering an ask, saving and taking an agenda action each cost one more tap. An ask opens before it is answered, so its price is read first. Save moves to a menu at the top, so it is never under a scroll. The agenda folds; in quick mode it stays open (§4.4) |
| **A** | Least gain for the notebook: two rows of tabs and a search box stay, so the first person starts 37% down. Town facts still live in two places and two tabs are still called Places. Today still mixes doing with standing |
| **B** | The place list is one level deeper: a tap more unless the pocket was left there. Town facts leave the top level. DEC-088's four tabs are replaced. Five specs name the old tab ids directly |
| **C** | The map covers the bar (`styles.css:636`, `:655`), so End turn is a close away while browsing. The rows of a list get the last 37% of the screen. Policies and the rival's plans have no place on a map. It waits for the map layers (M6 note 11). It is the largest build, and the bar's tuned layout changes |

### 2.3 Where each new screen lives

| Screen | A | B | C |
|---|---|---|---|
| **Arc view** (`STAGED_ARCS.md` A3) | A row under "Under way" on Today opens the arc sheet | The same | The same; the arc is also a map layer, its beats numbered pins |
| **Exchange scene** (DEC-112, `EXCHANGE_SCENES.md` §7) | In the conversation view, as designed. Reached from the arc sheet's "What you can do", and from a "Needs you" row on the days it is open | The same | The same; its place is marked on the map |
| **Bill impact sheet** (`IMPACT_ESTIMATE.md` §6) | A bill's row under "Under way" opens the four lines; the full sheet opens from them | The same; the bill is also listed under Notebook, Policies | The same; "Show on map" is one tap, the layer being next door |
| **Map layers** (note 11) | On the map. Rows in the notebook link to them ("On the map") | The same | The layers are the reference menu |
| **"Why?"** (built) | The panel as built. New openers: the arc's story lines, a settled ask, a bill's row | The same | The same |
| **Career card** (note 23) | More menu, "Legacy and career card" | The same | The same |

In all three a detail opens as its own sheet over the pocket on a phone, as decisions do today, and closes back to the row that opened it.

---

## 3. The parts common to all three

### 3.1 The person row

**Decision: two lines, and a third when the person matters now.** One line a person was the direction put to the owner. It does not fit: at 393 px a single line cut the role to "Relief orga…" and the reason at 30 characters. Three short lines read whole.

| Line | Holds | Example |
|---|---|---|
| 1 | The name; "New" when learned this turn | **Nell Harrow** New |
| 2 | How they stand with the player in a word with a shape, then the role | ▲ Warm · Relief organiser |
| 3, only when they matter now | The reason, 34 characters at most | Asked you for a second roof |
| At the end | One action: "Talk" when they are here, "Go" when they are not | Go |

- A row is 57 px, or 76 px with a reason. Today's card is about 205 px.
- A pixel portrait leads the row (`web/src/components/portrait/Portrait.tsx`), so a face ties the list to the street and the map (note 11).
- The stance is a word and a shape (▲ ◆ ▼), never colour alone (DEC-051). It needs five short Fluent lines beside the long ones ("Warm" beside "Warm toward you").
- Tapping the row opens it in place. Tapping the action acts at once: "Go" walks there, as "Go to them" does now (walking is free, DEC-079).

### 3.2 The row, opened

It opens in place, one at a time, 452 px in the mock-up (0.7 of a screen). In order:

1. **Why now**, in full, with a link to the ask, arc or decision.
2. How they stand: the opinion and trust lines, and favours owed either way.
3. What the player knows, as facts: what they do, what they care about, what they want once that is known.
4. What is not known, **only when it changes something here** (§3.4).
5. Where: "Usually at the community centre in the afternoon."
6. Talk, Go to them, On the map.
7. "Everything about Nell Harrow": the character sheet as built, for memories, favours and how each thing was learned.

**Reason:** today the card shows everything and the sheet repeats it without Talk or Go (`CharacterSheet.tsx` has only "Go here"). Now the row decides, the opened row acts, and the sheet is the archive.

### 3.3 "Matters now"

**The engine orders; the UI only lays out.** Who matters is a reading of asks, arcs, decisions, votes and where people stand. The UI holds none of that authority (rule 4).

A person matters now when the player knows them and any of these holds:

| Reason | True when |
|---|---|
| `ask` | They asked, or are named in, an ask or promise that is offered or accepted and not settled |
| `arc` | They are in the cast of a running arc in the player's focus, and an open beat names them |
| `decision` | They are among the people of a pending decision |
| `vote` | A vote on the player's bill falls within the turn and, as far as the player knows, they are undecided |
| `here` | They are where the player is and can be spoken to |
| `rival` | They are the player's rival, during a campaign |

Order: the soonest deadline first, then people who are here, then by reason. At most eight are shown; the rest are one tap away ("3 more").

**Fields the engine supplies.** One new query replaces the per-card queries of §1.3.

```text
QueryDto::People -> PeopleView { rows: Vec<PersonRowView> }

PersonRowView {
  id, name, role_key, ward_number, look           // look: what the portrait needs
  opinion_band, trust_band, owes_you, you_owe     // as CharacterView gives them today
  group: now | office | rest | gone               // which section; `office` from the focus jurisdiction's posts (DEC-098)
  rank: u32                                       // order inside the group; the UI sorts by nothing else
  now: Vec<NowReasonView>                         // strongest first; empty unless group is `now`
  here: bool, can_talk: bool
  go: RouteView | null                            // where "Go" walks; today a talk query on every tap
  usually: SeenView | null, last_seen: SeenView | null
  note: TextView | null                           // knowledge in words, only where it changes something (§3.4)
  new_this_turn: bool
}
NowReasonView { kind: ask | arc | decision | vote | here | rival,
                key, args,                         // a Fluent line, 34 characters at most in English
                ask | arc | decision | bill: id,   // what the reason opens
                due: DateView | null, days_left: u32 | null }
```

The same pair, `now` and `rank`, goes on a place (`SpotView`): a scene waits there, an ask is done there, an arc's beat is there, the player is there.

For Today, `PocketView` gains two lists:

```text
needs_you: Vec<NeedView>    // ordered by the engine, soonest first
NeedView { kind: decision | ask_offer | ask_due | vote | scene | case,
           title_key, args, due, days_left, target, arc: id | null }
arcs: Vec<ArcSummaryView>   // id, title_key, args, place, clock (key, args, days_left), open_beats: u32
```

`needs_you.len()` is the bar's count. Today the UI adds three numbers up for it (`Pocket.tsx:204-214`).

Redaction is as for every view: a reason is never given about a person or a matter the player does not know (DEC-091). The world is the same whether or not the pocket is opened.

### 3.4 Knowledge, in words

The owner's principle (DEC-091, 2026-11-25): knowledge is not a checklist; nothing counts it up; the notebook is what the player understands. Today every card carries a three-dot meter (`notebookModel.ts:221`), and the notebook lists dashed placeholders for what is not known, "so the notebook doubles as an exploration guide" (`Notebook.tsx:5-8`).

**Decision: the level is never drawn. It shows as the facts it reveals, and as one sentence where a gap changes what the player can do.**

| The ledger says | Today | Reworked |
|---|---|---|
| Not known (a gap) | A dashed card, "○○○ Not known yet", with a lead | Not listed. A sentence where it bears on something in hand: an option, an ask, a bill in that ward. A search for it answers with the way to find out |
| Heard of | "●○○ Heard of" | The row shows what is known. One sentence where it matters: "You only know the name." |
| Known | "●●○ Known · Canvass a neighbourhood, 1 March" | Nothing. The facts are the display. Source and date are in the full sheet, under "How you learned this" |
| Known in detail | "●●● Known in detail" | The extra facts appear: what they want, whom they owe |
| A stale figure | "as of 3/2038 · old figures" | Kept, in a sentence beside the number: "Figures from March. They may have moved." Numbers carry their date (DEC-091) |
| A lead | "To find out: … (Would raise this to ●●○ Known)" and Go | "To find out: …" and Go, in the opened row. The clause in brackets goes; note 18 already disliked it |

- **Whether a gap changes something is the engine's to say.** It knows which actions are open and which of them turn on that subject. So the sentence is a field, `note` (a Fluent key and arguments), on a person, a place, a neighbourhood and a bill. It is empty when the gap changes nothing.
- **Leads stay** (DEC-091: "finding things is guided, not hunted"). A lead shows on the thing it is about. Those picked up in conversation are also together in one folded section, "Loose ends", under Town. The list of standing ways to fill every gap goes: it was the checklist.
- A scene's option whose size turns on something not learned still shows its range (DEC-112 clause 6). That is the same rule in another place.

### 3.5 Folded sections, with counts

- A section is a heading and its rows, or one folded row with its name and a count: "Everyone else you know · 43".
- **A count is how many rows are inside.** It is never a share: no "43 of 70", no bar, no per cent. Nothing has a count of what is missing.
- Open by default: "Needs you", "Under way", "Matter now", and the neighbourhood the player is in. Everything else is folded. A fold the player opens stays open for the session (a device preference, like the tab).
- A fold opened with more than twelve rows shows twelve, most recently seen first, then "Show the other 31".

### 3.6 Search

- One search for the whole notebook, as today (`Notebook.tsx:175-184` already counts hits in the other tabs).
- While a search is typed, the tabs give way to one flat list grouped by kind: people, places, worries, policies. Nothing is folded and no tab needs choosing. Today the player is told the other tabs hold hits and must go to each.
- It is a filter over text already on the device, as now. It decides nothing.
- On a phone the field is a glass that opens to full width (B) or a field (A, C). `/` moves the keyboard to it on a desktop.

### 3.7 Empty states

| Where | Says |
|---|---|
| Today, nothing waiting | "Nothing is waiting on you." and the next dated thing: "Next: the council votes on Transit frequency, 12 March." The engine names it (`next_up`: a key, arguments, a date) |
| Under way, nothing | The section is left out |
| People, a newcomer | "You don't know anyone here yet. People you meet are kept here." No list of whom to meet |
| Matter now, nobody | The heading is left out; "In office" opens instead |
| Search, no hit | "Nothing you know matches "…"." as now. For a gap: "You don't know what Eastvale worries about. Canvassing there would tell you." |
| Arcs, none running | No row. The More menu's "Story so far" still lists the ones that ended |

### 3.8 A thirty-year career

Assumed: 60 people known, 25 places, 12 asks open and past, 5 arcs seen.

| List | How it stays reviewable | Opening view |
|---|---|---|
| **60 people** | Matter now (7 in the mock-up, 8 at most), then three folded rows: In office · 6, Everyone else you know · 43, Moved on or retired · 4. Search reaches all 60 | 1.3 screens (B) |
| **25 places** | Eight neighbourhood rows, each with its worries and how many places; the player's own is open. A place with something waiting also shows on Today by its ask, arc or decision | About 1.2 screens (eight rows of 57 px and one open) |
| **12 asks** | Offers under "Needs you"; accepted ones under "Under way" by due date; the last four weeks' outcomes in "Settled lately", folded. Older ones are in "Story so far" with the tally, as now | Inside Today's 1.4 |
| **5 arcs** | Running ones are rows under "Under way". Ended ones are in "Story so far"; each reopens its arc sheet to read (a scene's rounds are kept: `EXCHANGE_SCENES.md` §7.7) | One row each |

If "Needs you" and "Under way" pass nine rows together, "Under way" folds its tail ("and 4 more"). The engine's order decides which show.

### 3.9 The arc sheet

`ArcView` is A3's. This is its layout, so the pocket and the arc are built to one pattern.

1. Head: the arc's name, where, how long it has run.
2. **The clock**, in a sentence: "The meeting at the hall is in 9 days."
3. **The story so far:** the latest stage's sentence, with "Why?". Earlier stages fold under "How it came to this".
4. **What you can do:** every open beat as a row with its place or person, its cost and one button (Choose, Answer, Go). The turn, when it is ahead, is the last row, muted, with its date.
5. **Who is in it:** the cast as person rows.
6. In the paper: the latest headline.

Heat is never shown as a bar (`STAGED_ARCS.md` §1.6). Below "known" the sheet shows a lead only (A3's acceptance).

---

## 4. Desktop, keyboard, screen reader, quick mode

### 4.1 Desktop (900 px and wider)

As built: the pocket is a 400 px panel on the right and can stay docked (`styles.css:403-405`, `:738-744`); every other sheet opens on the left (`styles.css:175-177`).

| | At 900 px and wider |
|---|---|
| **A** | As today. A person opens in place in the pocket. The arc sheet, a decision and "Why?" open on the left, so the row that opened them stays in view |
| **B** | **Both sides can be open at once.** Today docks on the right. The Notebook opens on the left like any other sheet. The switch becomes two buttons, each showing its own panel. A detail opening on the left covers the Notebook and gives it back when closed |
| **C** | Three columns: the layer's list on the left, the map in the middle, Today docked on the right. The map stops covering the bar |

Sheets are the full height of the window on a desktop, so the length budget (§5) binds on phones only.

### 4.2 Keyboard

| | Keys |
|---|---|
| **All** | A tab row is one Tab stop; arrows, Home and End move along it (as `PocketTabs.tsx` does now). A list is one Tab stop; Up and Down move between rows; Enter opens or closes a row; Tab then goes into the opened row's buttons. Escape closes the top sheet and puts the keyboard back on what opened it. `/` goes to search. The skip link to Places stays |
| **A** | Two tab rows in the notebook, as now |
| **B** | The switch is a tab row of two. In the Notebook the next Tab stop is its tab row |
| **C** | The layers are a tab row whose panel is the list. The list comes before the map's own buttons in Tab order. The map keeps its keys (DEC-086) |

### 4.3 Screen reader

- A row's name is its lines in a sentence: "Nell Harrow, relief organiser. Warm toward you. Asked you for a second roof." Its action is named in full: "Go to Nell Harrow" (`ui-people-go-to` exists).
- A row is a button with `aria-expanded`. A folded section is a button whose name carries its count: "Everyone else you know, 43 people".
- Sections are headings, so the pocket can be walked by heading.
- When a command reorders "Matter now", nothing is announced. The pocket's badge says what changed, as it does today.
- Opening a detail moves focus to its title; closing returns it (as `WhyPanel.tsx` does).
- With reduced motion a row opens at once.
- In C the list is the text alternative to the map, as DEC-086 requires, and it is the first thing reached.

### 4.4 Quick mode and Places parity (DEC-051, DEC-088)

Quick mode is not in the app yet (`web/src/state/devicePrefs.ts` has no setting for it; M6 checklist, Settings). It is "the town map plus place panels, with no walking". In every structure:

- Every place is in the place list as a button that sends the same `visit` as arriving on foot.
- The agenda performs every located action. In quick mode "Take an action" is open by default and sits second on Today.
- Every arc beat is in the arc sheet as a located action (`STAGED_ARCS.md` §5), and the exchange has its one-sheet form (`EXCHANGE_SCENES.md` §7.3).
- Nothing is reachable only by the map. C's map list is a list first.

| | In quick mode |
|---|---|
| **A** | As today: the Places tab is quick mode's home |
| **B** | The pocket reopens where it was left, so a quick-mode player who lives in Notebook, Places lands there. The skip link goes there directly |
| **C** | The map and its list are quick mode. This is C's best fit |

---

## 5. A length budget

Rules a test can hold. Sizes are at 393 × 793 unless said. "Screen" is the sheet's greatest height on that phone (85% of the window).

| # | Rule | B in the mock-up |
|---|---|---|
| L1 | The opening view of Today, of every Notebook tab and of the arc sheet is at most **2.0 screens**, with a mid-career run and the default folds | 1.4, 1.3, 1.8 |
| L2 | The first item starts at most **25%** down the sheet (40% in A) | 13% and 20% |
| L3 | With eight people who matter now, at least **5 whole people** are on the first screen | 6 |
| L4 | A closed row is at most 80 px; a folded section at most 60 px; an opened person at most 0.75 of a screen | 76, 46, 0.67 |
| L5 | With a decision waiting, its row is wholly on the first screen at **360 × 640** | At 393 px wide its row ends 298 px down; the sheet on a 640 px phone is 544 px |
| L6 | In the arc sheet, "What you can do" and its first three rows are on the first screen | Yes |
| L7 | No sideways scroll at 320 px; every target is at least 44 px (as `gamebar.spec.ts` holds for the bar) | Checked at 360 and 393 |
| L8 | **Nothing counts knowledge.** The pocket has no meter (`.nb-dots` is gone) and no text of the form "N of M known" or a per cent beside a knowledge word | None |
| L9 | Taps: a decision is answered in 3 activations from the street, a person who matters now is gone to in 3, an arc's actions are reached in 2, Save in 3 | As §2.1 |

**The checks.** One new spec, `web/e2e/pocket-budget.spec.ts`, on the phone project.

- **A mid-career run.** A `.mandate` file imported on the title screen, the way `run-transfer.spec.ts` imports one. The lab makes it from a pinned seed, and `scripts/repin.sh` makes it again when the save version moves. The spec first asserts what the file must hold (at least 50 people known, 20 places, a running arc, an offer, a decision) and fails loudly if it is thin, so a poor file can never pass the budget by being short.
- **L1, L2:** read the sheet's `scrollHeight` and the top of its first item; divide by 85% of the window's height.
- **L3, L5, L6:** count the rows whose box ends above the sheet's bottom edge before any scroll.
- **L4:** the tallest closed row; open one and measure it.
- **L7:** `document.documentElement.scrollWidth <= innerWidth` at 320 px, and the box of every button, as the bar's spec does.
- **L8:** the pocket's text against the patterns, on every tab.
- **L9:** play each path by keyboard and count activations, as the exchange's test counts a debate's (`EXCHANGE_SCENES.md` §7.6).
- **Words:** a reason line longer than 34 characters, or a note longer than 140, fails the content doctor's text lint by key prefix (`now-*`, `know-*`), as scene lines do.

The budget also runs once on a turn-4 run from a pinned seed, so an early game is held as well as a late one.

---

## 6. What it costs

### 6.1 Components

| File | A | B | C |
|---|---|---|---|
| `components/Pocket.tsx` (325 lines) | Today rewritten as rows; links to a menu | The same, and the switch replaces the tabs | The same, no tabs at all |
| `components/PocketTabs.tsx`, `state/devicePrefs.ts`, `state/gameStore.ts` | Unchanged | Two sides and a notebook tab remembered; the old four values map across (`devicePrefs.ts` already maps a removed tab) | Tabs removed; a remembered layer |
| `components/TownCard.tsx` | Rows; neighbourhoods fold | Becomes Notebook, Town; neighbourhoods move to Places | Becomes the Town layer's list |
| `components/PlaceList.tsx` | One line a place | The same, as Notebook, Places | The Places layer's list |
| `components/notebook/**` (People, Notebook, the model, the CSS) | Rows, folds, words | The same, four tabs | The same, as layer lists |
| `components/asks/Asks.tsx` | A row that opens to the form | The same | The same |
| `components/StorySheet.tsx`, new `ArcSheet.tsx` | A3, as §3.9 | The same | The same, and an arc layer |
| `components/talk/**`, new `components/exchange/**` | X2, unchanged by this note | The same | The same |
| `screens/Game.tsx`, the bar | Badges merged to two (question 5) | The same | The same, and a badge on Map |
| `components/TownMap.tsx`, `hub/townMapView.ts` | "On the map" links land on a layer | The same | A list drawer, layer tabs, pins: the large part |
| `styles.css`, `notebook.css`, `content/base/locale/en/ui.ftl` | Rows, folds, strip, menu; about 30 new strings | The same, and the switch | The same, and the drawer |

### 6.2 Tests

| | A | B | C |
|---|---|---|---|
| `e2e/pocket.ts` helpers | Insides only | Insides only: "town" and "places" open the Notebook on those tabs | Insides only: they open the map's list |
| Specs naming `pocket-tab-*` directly (5: `main-screen`, `notebook`, `keyboard-walk`, `talk`, `town-map`; 31 lines) | Unchanged | Edited | Edited |
| Specs using the links (11) | One helper, one more click | The same | The same |
| `notebook.spec.ts` (20 lines on the old ids and "Not known yet") | Rewritten for §3.4 | The same | The same |
| `main-screen.spec.ts` (the town card ids, 9 lines) | Light edits | Moved under the Notebook | Moved to the map |
| `gamebar.spec.ts` | Unchanged | Unchanged | Run again with the Map badge |
| New | `pocket-budget.spec.ts`, `arcs.spec.ts` (A3), `exchange.spec.ts` (X2) | The same | The same, and a map-list spec |
| Unit tests (`PocketTabs`, `Notebook`, `notebookModel`, `Asks`) | Rewritten with the components | The same | The same |

### 6.3 Protocol

All in `mandate-protocol`, with the TypeScript types generated from it. No state, no save version, no determinism risk: views only.

| Addition | For |
|---|---|
| `QueryDto::People` → `PeopleView`, `PersonRowView`, `NowReasonView` (§3.3) | The person row, its order and sections. Replaces one query a card |
| `PocketView.needs_you`, `NeedView` | Today's first section and the bar's count |
| `PocketView.arcs`, `ArcSummaryView` | An arc's row on Today (A3 plans Today items already) |
| `PocketView.next_up` | The empty state |
| `SpotView.now`, `SpotView.rank` | The place row's order and reason |
| `note` on a person row, a neighbourhood, a place, a bill, a notebook entry | Knowledge in words (§3.4) |
| `ArcView` (A3) and `ArcExchangeView` (X2), as their notes specify | Unchanged by this note |

The `level` stays in the views for tests; it is no longer drawn.

### 6.4 Build plan

Bundled so that **one web agent builds the pocket once, with the arc sheet (A3) and the scene's screen (X2)**. A3's and X2's web files join R2's `OWNS`, and their acceptance lines carry over word for word.

| # | Task | Tier | Kind | OWNS | After | Acceptance |
|---|---|---|---|---|---|---|
| R0 | The decision recorded; `UI.md` §1 redrawn | small | docs (lead) | `docs/design/decisions/`, `UI.md` | The owner's answers | The structure and the answers to §8 are on record |
| R1 | The views: people, needs you, arcs' summaries, `now` on places, notes; with A3's `ArcView` | mid | protocol | `mandate-protocol/src/{pocket,people,arcs,notebook}.rs`, the generated types, `locale/en/ui.ftl` (reason and note lines) | A1 (done). X1's types agreed | One query lists every person. The order is the same twice and through a save and load. No reason names a person or a matter the player does not know. The same commands with every subject known and with the default give the same world (K4 holds) |
| R2 | The pocket, the arc sheet and the scene's screen | large | web, one agent | `components/{Pocket,PocketTabs,TownCard,PlaceList,StorySheet,ArcSheet}.tsx`, `components/{notebook,asks,talk,exchange}/**`, `screens/Game.tsx`, `state/{devicePrefs,gameStore}.ts`, `styles.css`, `e2e/{pocket.ts,pocket-budget,arcs,exchange,main-screen,notebook}.spec.ts` and the specs of §6.2 | R1's types, in parallel against fixtures of them | The budget L1 to L9. Every helper in `pocket.ts` keeps its signature. A3: every beat kind taken from quick mode in one test; below "known" a lead only. X2: a debate by keyboard in 12 activations or fewer; five options at 360 px without scrolling. The accessibility check passes |
| R3 | The mid-career run file and its re-recording | small | lab, e2e | `web/e2e/fixtures/`, a `scripts/repin.sh` step | Any time before R2 ends | The file is made by one command and loads; the spec's preconditions hold |
| R4 | The registers for reason and note lines; the lint by prefix | small | docs, lint (with A9) | `STYLE_BIBLE.md`, `mandate-cli/src/doctor/text.rs` | Any time | Every `now-*` line is 34 characters or fewer; every `know-*` line is a whole sentence |
| R5 | "On the map" from rows; then C's list under the map | small, then mid | web (with note 11's map layers) | `components/TownMap.tsx`, `hub/townMapView.ts` | The map layers | A row's link opens the map on its layer, centred. Later: each layer has its list, reached before the map by keyboard |

R2 is built in four steps, each committed: the shell, Today and the More menu with the budget spec; the Notebook's tabs, rows and search; the arc sheet and its rows on Today; the scene's screen. For A the same plan holds with R2 one tier smaller in its first two steps. For C, R5's second half moves into R2 and R2 waits for the map layers.

---

## 7. Recommendation

**Build B.**

- **It answers the complaint most directly.** The notebook becomes one book with one row of tabs. Six people are on the first screen where today there is one; the first of them starts 20% down the sheet, not 47%. A keeps the second row of tabs and manages 37%.
- **It gives each new screen an obvious home.** Things to do (a decision, an ask, an arc, a bill, a scene) are rows on Today that open a sheet. Things to look up are in the Notebook. The next new screen will not need a fifth tab.
- **It removes the two muddles the count found:** two tabs called Places, and town facts kept in two places.
- **It does not wait.** C is the better long-term picture for browsing and for a desktop, but it needs the map layers and the largest build. B takes C's best part as its last step (R5): the rows link to the map now, and the map gains its lists when the layers are in.
- **Its costs are known and small:** a tap more to the place list unless the pocket was left there; DEC-088's four tabs replaced; five specs edited by hand.

If the owner would rather not move the tabs, **A is a sound fallback**: it brings Today from 4.4 screens to 1.4 and People from 20.7 to 1.5 with the common parts alone, and it can become B later without redoing them.

---

## 8. Open questions for the owner

| # | Question | Recommendation |
|---|---|---|
| 1 | Which structure? | **B.** A if the four tabs must stay. C's map lists as a later step, with the map layers |
| 2 | Do the dashed "Not known yet" placeholders and the three-dot meter go from the lists? | **Yes, both.** A gap is said in a sentence only where it changes something in hand, and a search for it answers with how to find out. Leads picked up stay, as "Loose ends". This is the 2026-11-25 principle applied to the notebook |
| 3 | Save, the story, the maps and the rest: a "More" menu at the top, or links at the foot of the page as now? | **The menu.** Three taps and no scrolling, in one known place. At the foot they are a tap fewer and a scroll more on every page |
| 4 | On a desktop, may the Notebook sit on the left with Today docked on the right, both open? | **Yes.** It is what two sheets side by side are for, and it costs little once the two sides are separate panels |
| 5 | The bar can show four badges (decisions, asks, alerts, new in the notebook). Merge them to two: what needs you, and what is new? | **Yes.** One number for what waits is what Today's first section shows. Drawn with the app's CSS, four badges did not fit the 44 px button on a narrow phone (§1.3); a test should confirm it either way |

---

## 9. Decision proposal

## DEC-113 — The pocket: Today and the Notebook

- **Status:** Proposed (2026-11-25). The number is the lead's to confirm.
- **Context:** the owner found the notebook too long to review on a phone. Counted: Today is 2.9 to 4.4 screens, People 20.7 screens at 60 people known, and the notebook's first card starts 47% down the sheet (`POCKET_REWORK.md` §1).
- **Decision:**
  1. **Two sides.** The pocket is Today, for doing, and the Notebook, for reference, under one switch. The Notebook has one row of tabs: People, Places, Town, Policies. This replaces the four tabs of DEC-088 "Main screen"; the rest of DEC-088 stands.
  2. **Rows.** A person, a place, an ask, an arc and a bill are each a row of two or three short lines that opens in place. Detail is a sheet over the pocket.
  3. **What matters now comes first, and the engine says what that is.** Views carry the order and the reason. The UI sorts and counts nothing of its own (rule 4).
  4. **Knowledge is shown in words** (DEC-091, 2026-11-25). No meter and no list of gaps. A gap is a sentence where it changes what the player can do, supplied by the engine. Leads stay.
  5. **Folded sections carry a count of what is inside, never a share.**
  6. **A length budget is tested** (`POCKET_REWORK.md` §5): no opening view over two phone screens; nothing counts knowledge.
  7. **Parity holds** (DEC-051): every place, action, beat and scene is reachable from lists, without walking and without the map.
- **Order:** R1 with the arc views; R2 as one web task with A3 and X2; R3 and R4 beside them; R5 with the map layers.
- **Dependencies:** DEC-051, DEC-079, DEC-086, DEC-088, DEC-091, DEC-093, DEC-098, DEC-109, DEC-112.
