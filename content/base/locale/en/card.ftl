## The career card and stepping away (M6 note 23). The engine gives the
## facts and keys (`CareerCardView`, `WorldCodeCheckView`); the client words
## them here, for the card on screen, its copied text and its image.

## Stepping away: the ending (endings.jsonc) and the Legacy record's lines.
ending-stepped_away = Stepped away
ending-stepped_away-text = You stepped away from politics after { $years ->
    [0] less than a year
    [one] a year
   *[other] { $years } years
}. Nobody made you go. The record stops here, unfinished.
legacy-stepped_away = Stepped away from politics.
legacy-stepped_away-race = Stepped away from politics, walking away from the race for { $post }.
legacy-arc_ended = { $title }: { $ending }

progress-card = Measuring how your career began…

## The step away, from the ⋯ menu: one confirmation.
ui-step-away = Step away from politics
ui-step-away-title = Step away from politics?
ui-step-away-body = Your career ends here, unfinished. The record closes with today's date and the post you hold, and you get your career card. This cannot be undone.
ui-step-away-race = A race you are standing in is under way: you will walk away from it.
ui-step-away-yes = Step away
ui-step-away-no = Keep going

## The card.
ui-card = Career card
ui-card-open = Your career card
ui-card-unfinished = Unfinished
ui-card-loading = Making your career card…
ui-card-who = { $name } · { $party } · { $town }
ui-card-who-independent = { $name } · independent · { $town }
ui-card-world = A { $size } world · { $scenario } · { $constitution }
ui-card-world-random = a random country
ui-card-size-small = small
ui-card-size-medium = medium
ui-card-size-large = large
ui-card-opening = Opening
ui-card-opening-arc = { $arc }, in { $town }
ui-card-opening-plain = { $town }, as the world was made
ui-card-opening-ahead = { $n ->
    [0] from the first day
    [one] a month on
   *[other] { $n } months on
}
ui-card-rating = rated { $pct } ({ $wins } of { $plays } plays won)
ui-card-rating-none = not rated
ui-card-fact = { $factor } { $margin }
ui-card-races = Races
ui-card-races-none = No race entered.
ui-card-race = { $year }: { $post }, { $place }
ui-card-race-won = won with { $share }
ui-card-race-lost = lost with { $share }
ui-card-race-walked_away = walked away from
ui-card-race-undecided = not decided
ui-card-highest = Highest post
ui-card-highest-none = No post held
ui-card-highest-post = { $post }, { $place }
ui-card-years = { $years ->
    [one] { $years } year served
   *[other] { $years } years served
}
ui-card-decisions = Notable decisions
ui-card-decisions-none = None recorded.
ui-card-arcs = How each matter ended
ui-card-arcs-none = No matter of yours came to an end.
ui-card-arc-under-way = still under way
ui-card-ended = How it ended
ui-card-ended-as = { $date }, as { $post }
ui-card-ended-out = { $date }, out of office
ui-card-figure = Wins over expected
ui-card-figure-value = { $over } ({ $wins } won of { $rated } rated; { $expected } expected)
ui-card-figure-none = No rated race was decided.
ui-card-code = World code
ui-card-code-none = This run cannot be played again from a code.
ui-card-code-help = Paste it on the title screen to play the same opening.
ui-card-build = Build { $build }
ui-card-more = and { $n } more
ui-card-copy = Copy
ui-card-copied = Copied.
ui-card-share = Share
ui-card-shared = Shared.
ui-card-saved = Image saved, and the text copied.
ui-card-saved-only = Image saved.
ui-card-copy-failed = Could not copy. The text is below to select.
ui-card-text-title = Mandate career card
ui-card-text-title-unfinished = Mandate career card (unfinished)
ui-card-image-alt = Your career card as an image

## Starting from a world code (the title screen).
ui-code-start = Start from a code
ui-code-label = World code
ui-code-read = Read the code
ui-code-begin = Play this world
ui-code-other-build = This code was made with another build of the game: the world may not be the same.
ui-code-world = A { $size } world · { $scenario }
ui-code-opening = Opening: { $arc }, { $ahead }
ui-code-opening-none = Begins as the world was made.
ui-code-knowledge = Starts with local knowledge.
ui-code-error-malformed = That is not a world code.
ui-code-error-version = That code is from a later version of the game.
ui-code-error-check = That code has a typo in it.
ui-code-error-unknown = That code names something this build does not have.
ui-code-error-other_build = That code is from another build of the game, and names something this one does not have.
