## The act flow (DEC-005, DEC-012, DEC-006): Term in review, time skips,
## interrupts, the homecoming summary, act endings and the Legacy record.

## Directives (act_flow.jsonc)
directive-focus = Focus
directive-focus-help = The worry your doorstep work and new bills aim at.
directive-focus-known_top = Whatever people worry about most
directive-vote = Votes on bills
directive-vote-help = How you vote on the bills before your body while you are away.
directive-vote-with_party = With the party
directive-vote-conviction = By conviction
directive-vote-ward = By what your voters think
directive-ground = Ground game
directive-ground-help = Doorsteps and public meetings around your ward.
directive-fundraising = Fundraising
directive-fundraising-help = Raising money (and, in a campaign, spending it on ads).
directive-press = The press
directive-press-help = Statements to the local paper.
directive-legislation = Bills
directive-legislation-help = Drafting a bill people want, and making the case for it.
directive-effort-none = None
directive-effort-some = Some
directive-effort-lots = A lot
directive-caution = Caution
directive-caution-help = Whether to take risks, and when to rest.
directive-caution-careful = Careful: no shady options, rest early
directive-caution-bold = Bold: small risks, work on while tired

## Skip targets
skip-target-next_session = The next session
skip-target-campaign = The campaign
skip-target-election = Election week
skip-target-end_of_term = The end of the term

## Interrupts: why a skip stopped
interrupt-status = Your situation has changed.
interrupt-scandal = A story about you has broken.
interrupt-decision = A decision needs you.
interrupt-election = The campaign has begun, and you are on the ballot.
interrupt-sponsored_vote = Your bill comes to a vote { $days ->
    [0] today.
    [one] tomorrow.
   *[other] in { $days } days.
}
interrupt-request_due = Someone is waiting on your answer.
interrupt-standing = Your standing has fallen to { $value }.
interrupt-viability = Your party is losing faith in you (viability { $value }).
interrupt-legal = The legal danger is rising ({ $value }).
interrupt-threat = Your safety is at risk ({ $value }).
interrupt-health = You are close to exhaustion (health { $value }).

## Endings (endings.jsonc). Arguments: the facts by key.
ending-jailed = Behind bars
ending-jailed-text = Your career ended in a courtroom after { $years ->
    [one] a year
   *[other] { $years } years
} in politics.
ending-removed = Removed from office
ending-removed-text = A conviction cost you your seat. The record stops here.
ending-disgraced = Disgraced
ending-disgraced-text = Nobody would stand beside you any more. Your standing was gone.
ending-higher_office = The next rung
ending-higher_office-text = You won higher office after { $years ->
    [one] a year
   *[other] { $years } years
}. Act 1 is over.
ending-executive = In charge
ending-executive-text = You won the top office at home after { $years ->
    [one] a year
   *[other] { $years } years
}. Act 1 is over.
ending-deselected = Dropped by the party
ending-deselected-text = Your party would not put you on the ballot again.
ending-lost_seat = Voted out
ending-lost_seat-text = The voters chose someone else. After { $years ->
    [one] a year
   *[other] { $years } years
}, you are out of office.
ending-served = A full stint
ending-served-text = You served out { $elections_won ->
    [one] your term
   *[other] { $elections_won } terms
} in office.

## The Term in review screen, skip progress, summary and epilogue
ui-term-review = Term in review
ui-term-review-open = Skip ahead
ui-term-review-intro = Set how you want things run while you are away, then choose how far to skip. The town goes on as normal; you are called back when something needs you.
ui-term-review-directives = Directives
ui-term-review-target = Skip to
ui-term-review-skip = Skip
ui-term-review-none = There is nothing to skip to now.
ui-skip-target-date = { $name } ({ $date })
ui-skipping = Skipping ahead…
ui-skipping-progress = { $date } · { $turns ->
    [one] 1 turn
   *[other] { $turns } turns
}
ui-skip-cancel = Cancel
ui-skip-summary = While you were away
ui-skip-summary-span = { $from } to { $to } · { $turns ->
    [one] 1 turn
   *[other] { $turns } turns
}
ui-skip-stop-reached = You reached { $target }.
ui-skip-stop-cancelled = You stopped the skip.
ui-skip-stop-limit = That is as far as one skip goes.
ui-skip-stop-act_ended = The act has ended.
ui-skip-changes = What changed
ui-skip-happened = What happened
ui-skip-nothing = Nothing of note.
ui-skip-continue = Back to work
ui-skip-change-mandate = Mandate
ui-skip-change-standing = Standing
ui-skip-change-viability = Viability
ui-skip-change-legal_jeopardy = Legal jeopardy
ui-skip-change-health = Health
ui-skip-change-money = Money
ui-skip-change-population = Residents
ui-skip-change-unemployment = Unemployment
ui-skip-change-laws = Laws in force
ui-skip-change-executive = A new holder of the top office: { $name }
skip-item-election-won = You won the election for { $post }.
skip-item-election-lost = You lost the election for { $post }.
skip-item-law-mine = Your bill became law: { $policy }.
skip-item-law = A new law at home: { $policy }.
skip-item-bill-failed = Your bill failed: { $policy }.
skip-item-answered = Decided for you: { $title } ({ $option }).
skip-item-defaulted = Left to its default: { $title }.
skip-item-scandal = A story broke about you.
skip-item-chapter = A new chapter: { $kind }.
skip-item-disaster = A disaster struck the town.
skip-item-plant-closure = A plant closed: { $jobs } jobs lost.
skip-item-voted = You voted { $vote } on { $policy }.
ui-epilogue = Epilogue
ui-epilogue-open = The act is over: read the epilogue
ui-epilogue-legacy = Legacy
ui-epilogue-facts = The record
ui-epilogue-final = Final standing
ui-epilogue-new-game = New game
ui-epilogue-close = Close
epilogue-fact-posts_held = Posts held
epilogue-fact-years = Years
epilogue-fact-laws_passed = Laws passed
epilogue-fact-works = Works
epilogue-fact-elections_won = Elections won
epilogue-fact-elections_lost = Elections lost
epilogue-fact-promises_kept = Promises kept
epilogue-fact-promises_broken = Promises broken
epilogue-fact-decisions = Notable decisions
epilogue-fact-scandals = Scandals
epilogue-fact-mandate = Mandate
epilogue-fact-standing = Standing
epilogue-fact-viability = Viability
legacy-post_taken = Took up the post of { $post }.
legacy-post_left = Left the post of { $post }.
legacy-election-won = Won the election for { $post } ({ $share }).
legacy-election-lost = Lost the election for { $post } ({ $share }).
legacy-law_passed = Passed { $policy }.
legacy-bill_failed = Failed to pass { $policy }.
legacy-decision = Decided: { $title } ({ $option }).
legacy-promise_kept = Kept a promise to { $name }.
legacy-promise_broken = Broke a promise to { $name }.
legacy-scandal = A story broke about you.
legacy-case = The case against you: { $stage }.
legacy-removed = Removed from office by a conviction.
legacy-deselected = Dropped by the party.
legacy-imprisoned = Sent to prison for { $years ->
    [one] a year
   *[other] { $years } years
}.
legacy-released = Released from prison.
ui-skip-open-decision = See the decision
