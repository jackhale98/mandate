# Fallible answers: beliefs with a source and a confidence

**Status:** proposed design (M6 note 27, second part, 2026-11-25). Not built. It changes DEC-091's knowledge ledger, so it needs the owner's approval before any task starts. The first part of note 27 (the broker leans on votes, backs a candidate, and can be lobbied, done a favour, opposed or exposed) is built and is what a wrong belief would cost against.

The owner's wish: people may be wrong about who runs things. Today every answer is true or a miss. A person either tells the player the truth, at a depth their trust allows, or says they do not know. This note lets some answers be wrong, in ways that make sense, and makes the player's notebook hold what they believe and why.

## 1. The rule in one paragraph

A few subjects can be believed wrongly: who runs a town, the rival's plans, and gossip about a person. Everything else stays reliable. When someone answers on one of these subjects, they say what they believe. What they believe is right more often the closer they are to power. When it is wrong, it is wrong in the way their position makes likely: they name the visible power, their own boss, or someone they resent. The player's notebook records each answer as a claim with its source. Two independent sources that agree raise the player's confidence. Acting on a wrong belief costs what the action costs, without the benefit, and the outcome shows it was wrong.

## 2. What can be wrong, and what cannot

**Fallible (three subjects):**
- **Who runs things** (`Subject::Broker`): the claim is a person.
- **The rival's plans** (`Subject::RivalPlans`): the claim is the post the rival will stand for, or that they will not stand.
- **Gossip about a person** (the open question's gossip, `talk::open`): the claim is one fact about them from a short list: who they are close to, who they owe, whether they have something to hide.

**Reliable (everything else):** the town's and a neighbourhood's worries, support for a policy, places, industries, sites, services, a person's job and post, the rules, the player's own state, and anything aimed at the player (DEC-091's never-gated list). Numbers keep their dated readings. A wrong number would be noise, not a story.

## 3. How people get it wrong

Errors are systematic, not random. The same person gives the same wrong answer all month, and their wrong answer follows from where they stand.

**Accuracy by closeness to power.** Each speaker has a closeness to power on the subject, from public facts and their own ties:
- the level `knows::expected_knows` already gives them on the subject (unknown, heard, known, detailed);
- whether they hold a post, lead an organisation, or work at the broker's organisation;
- their own relationship with the broker or the rival (an edge, either way, means they have dealt with them).

The chance their belief is right is `base + closeness × slope`, clamped (content parameters, `talk_params.fallible`). It is drawn once per speaker, subject and month, keyed (DEC-041). No stream moves, and asking again the same month gives the same answer.

**Who a wrong answer names.** The first that applies, in a content order:
1. **Their own boss:** someone who works at an organisation names its leader (a shop steward names the union steward; a reporter names the editor; a nurse names the head of the clinic where there is one).
2. **Someone they resent:** the person in town they think least of, if below a threshold. Resentment makes people see a hidden hand.
3. **The visible power:** the holder of the jurisdiction's executive office, from the constitution (DEC-098: never an office named in code).

For the rival's plans, a wrong answer is the rival's present post (people assume they will defend it) or the most obvious step up. For gossip, a wrong answer is the most common rumour kind for the person's role (an officeholder "owes someone"; a leader "is close to" the visible power).

**The broker stays coy.** Asked who runs things, the broker never names someone else in this first build. They deflect, as the existing lines do (`line.runs.coy`, `line.runs.broker_cool`). See question 1.

**Trust is willingness; closeness is accuracy.** The trust steps from note 26 (`talk_params.discretion`, `reply::deepest`) still decide how much a person tells. Closeness decides whether what they tell is right. A trusting speaker far from power tells the player plainly, and is plainly wrong.

## 4. Confidence

The player never sees a number. Confidence has three words:
- **one person says** (a single source, or several from one circle);
- **people agree** (two independent sources name the same claim);
- **you have seen it** (first-hand: the broker admits it at depth, the player watched them deliver votes, the rival filed).

**Independent** means: not the same person, not working at the same place, and not of the same organisation. The second source did not just repeat the first. This needs no record of who told whom.

**Disagreement is shown.** When sources disagree, the notebook lists the claims side by side, each with its sources ("the steward says Ada Hale; the shopkeeper says the mayor"). The player's belief is the claim with the most independent sources (ties: the most recent). A first-hand source beats any number of second-hand ones.

**Claims go stale.** A claim about who runs things is dated. When the broker changes (the town character is recomputed monthly), old claims are not corrected by magic. They are simply older, and the notebook says when each was heard.

## 5. Acting on a wrong belief

Every action that uses a belief acts on what the player believes, shows the confidence next to its price (DEC-073), and pays its normal price. If the belief is wrong:

| Action | What happens when the belief is wrong |
|---|---|
| Lobby "the broker" | The time and the favour are spent; the person has no pull, so no member moves. They say they cannot help. |
| Do them a favour | They owe the player a favour, but they cannot deliver the town. |
| Oppose them openly | The person is offended; the real broker is untouched; people who know better think the player foolish (a small loss of approval among those close to power). |
| Expose them through the press | The story falls apart: the press's misfire happens for certain, with its approval cost. |
| Plan against the rival's move | The player prepares for the wrong race (files, campaigns, asks) and the real rival is unopposed. |
| Repeat gossip | The person gossiped about hears of it and resents the player; the gossip's source is remembered. |

The outcome tells the player the claim was wrong. The ledger marks that claim **refuted** with the date and the action. It does not reveal the true answer; the player now knows who it is not. This keeps DEC-091's principle: knowledge changes decisions, and not looking is still a workable path (a player who never asks simply does not have the broker options; they still have lobbying member by member).

## 6. What changes in DEC-091's ledger

**Today.** `KnowledgeLedger.entries: BTreeMap<Subject, Entry>`. An `Entry` has a `level`, the first time each level was reached with its `Source`, an optional dated `Reading`, and the level the notebook has shown. `Subject::Broker { town }` holds no person. Every view reads who the broker is from live state (`Knowing::broker()`), so the ledger cannot hold a wrong belief at all.

**Proposed.**
1. **A claim list on the fallible entries.** `Entry` gains `claims: Vec<Claim>` (empty and left out of the save for every other subject). A `Claim` is a value (a person; a post or "not standing"; a gossip fact), its sources (each a `Source` with its date and the speaker's circle for independence), and `refuted: Option<(SimDate, Source)>`.
2. **Level means how much was heard.** `level` stays, and keeps meaning what it means for the trust steps: heard of a name, been told plainly, been told the detail. It no longer implies the answer is true.
3. **The belief is derived, never stored.** `Knowing::broker()` becomes `Knowing::believed_broker() -> Option<(CharacterId, Confidence)>`, computed from the claims by §4's rule. Nothing else stores a belief, so there is one place that decides it.
4. **Views read the belief.** The town character view, the notebook, the talk panel's "about" names, the broker openings (§5) and the bill and campaign fields added by note 27's first part (`BillView.broker_pull`, `CandidateView.backed_by_broker`) read the believed broker, with its confidence word. A view never shows the true broker to a player who believes otherwise. Gate (`mandate-protocol` knowledge redaction) does the reading, as today.
5. **The notebook.** Each fallible subject lists its claims with sources and dates, the confidence word, and refuted claims struck through with what showed it. Leads stay as they are.
6. **Lab switches.** `lab_omniscient` makes every belief the truth, "you have seen it". DEC-091's guard K4 (an omniscient player and a default player make the same world, except the ledger and the player's own outcomes) still holds: beliefs never gate what happens.
7. **Saves.** Format v31 → v32. The cheapest migration: every `Broker` and `RivalPlans` entry at `known` or above gets one claim of the live truth, with a `Migrated` source at "people agree"; entries at `heard` get no claim (a rumour without a name). Only the knowledge key renumbers; no other state moves. The migration reads live state at load, which `knowledge::on_load` already does.

## 7. What the conversations work already provides

Note 26's work does most of the plumbing:
- **`expect::who_knows`** finds the person the speaker knows who would know a subject best, from public facts. It becomes the way to a second, independent source.
- **`LeadToDef::WhoKnows`** already turns a miss into a lead to that person. A low-confidence claim gets the same lead ("ask someone at the union hall whether that is right").
- **The trust steps** (`talk_params.discretion` and `reply::deepest`) keep deciding depth. Fallible answers add a second axis (accuracy) without touching them.
- **The visible-power lines** (`line.runs.visible.*`, with `$office` and `$chamber` from `Ctx::role_args`) already name the visible power as a safe answer. Under this design they record a claim naming the holder of the executive office, at "one person says", instead of a miss.
- **`talk::open` gossip** already picks, by a keyed draw per week, someone the speaker would talk about. It becomes the gossip claim's subject.
- **`knows::expected_knows`** and the `is_broker` and `tells` conditions give closeness and the broker's coyness without new conditions.

What is new: the claim list and its migration, the speaker's belief (accuracy and the error order), lines that teach a claim (`teaches` gains `claim: "speaker_belief"`), and the outcomes that refute.

## 8. Measures

The grader and the doctor report, and the tuning pass settles (rule 11):
- the share of wrong answers by closeness band (more from the far, fewer from the near);
- how often a stand-in that acts at "one person says" pays for a wrong belief, against one that waits for "people agree";
- that no knowledge-gated option is best in every world (DEC-091 principle 7);
- K4 and determinism, as for every ledger change.

## 9. Task plan

1. **FA1, engine ledger** (`player/knowledge.rs`, `mandate-save`): claims, independence, the derived belief, refutation; save v32 with the migration above; K4 and save tests. No behaviour change yet: every claim taught is the truth.
2. **FA2, speakers' beliefs** (`characters/knows.rs`, `talk/`): closeness, the keyed accuracy draw, the error order, content parameters `talk_params.fallible`; lines teach `speaker_belief`; the visible-power lines and gossip record claims. Tests: the error falls with closeness; the same speaker, same answer all month.
3. **FA3, acting on beliefs** (`actions/broker.rs`, the rival's readers): the openings act on the believed broker and refute on failure; confidence shown with each price. Tests: each wrong-belief row of §5.
4. **FA4, views and notebook** (`mandate-protocol` gate and notebook, then `web/`): the confidence word, the claims with sources, refuted claims. A web task after FA3.
5. **FA5, lab** (`mandate-cli` grader and doctor): §8's reports; bots learn by the same rules.

FA1 to FA3 are one engine task in sequence; FA4 follows; FA5 can run beside FA4.

## 10. Questions for the owner

1. **May the broker lie?** When they distrust the player, they could name the visible power on purpose. *Recommendation:* not in the first build. Coy is enough, and a lying broker makes the one person the player most needs to read unreadable.
2. **How is confidence shown?** *Recommendation:* the three words of §4 and the sources by name, never a percentage. A number invites counting (DEC-091 principle 6).
3. **Does a failed action reveal the truth?** *Recommendation:* no, it only refutes the claim acted on. The player learns who it is not, and has to ask again. It keeps the search a choice.
4. **What does "Start with local knowledge" give?** *Recommendation:* true claims at "people agree" for the town's worries and the council, and still nothing on the broker. That matches DEC-091's background rule ("knows the council by name, but not who really runs things").
5. **Is three subjects the right scope?** Who supports a policy could also be fallible (a councillor overstating their side). *Recommendation:* keep three for now. Policy support has numbers and a whip, and an error there would read as a bug, not a story.
