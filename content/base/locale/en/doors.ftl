## Doors (DEC-114 §2, BEING_THERE §3): households behind the doors of a
## street, the doorstep exchange, and what a household remembers. B6 writes
## the full set of lines; these are the first.

## Who answers, and what they say first.
door-open-answers = { $given } { $family } comes to the door.
door-open-not-in = No answer at number { $number }. The { $family } family are out.
door-open-vacant = Number { $number } stands empty. The curtains are gone and the post is piling up.
door-work-plant-running = “I work at { $plant }.”
door-work-plant-short = “I work at { $plant }, when it’s open. Short time again.”
door-work-plant-notice = “I work at { $plant }. For now. You’ll have heard about the notice.”
door-work-plant-closed = “I worked at { $plant } till it shut.”
door-work-sector = “I work in { $sector }.”
door-work-retired = “I’m retired, love. I’ve the time if you have.”
door-work-looking = “I’m between jobs at the moment.”
door-home-owner = { $n ->
        [0] They own the house.
        [one] They own the house; a child’s bike leans by the step.
       *[other] They own the house; { $n } children’s coats hang in the hall.
    }
door-home-tenant = { $n ->
        [0] They rent.
        [one] They rent; a child’s bike leans by the step.
       *[other] They rent; { $n } children’s coats hang in the hall.
    }
door-mood-warm = They seem glad it is you.
door-mood-neutral = They wait to hear what you want.
door-mood-cool = They keep the door half shut.
door-mood-stranger = They don’t know who you are.
door-again = “You came round in { $month }.”
door-again-worry = “You came round in { $month } about { $worry }.”
door-again-poster = “You came round in { $month }. The poster’s still up.”
door-again-vote = “You came round in { $month }. I said you’d have my vote.”

## Their reply (the knock's outcome).
door-reply-not-in = You push a card through the door.
door-reply-vacant = Nobody lives here to remember the knock.
door-reply-listen = “It’s { $worry }, mostly,” { $given } says. “Nobody listens.”
door-reply-listen-none = { $given } shrugs. “Nothing much. We get by.”
door-reply-case-strong = You talk about { $issue }. { $given } nods along: “That’s it exactly.”
door-reply-case-fair = You talk about { $issue }. { $given } hears you out.
door-reply-case-weak = You talk about { $issue }. { $given } looks past you: it is not what keeps them up.
door-reply-ask-vote-yes = “You’ve got my vote,” { $given } says.
door-reply-ask-vote-no = “We’ll see,” { $given } says. “I haven’t decided.”
door-reply-ask-poster-yes = { $given } takes a poster for the window.
door-reply-ask-poster-no = “Not in the window, no,” { $given } says. “But good luck.”
door-agreed = Door after door says the same: you know what this neighbourhood is worried about.

## The street and the doorstep (the pocket's rows, the door sheet).
ui-doors = Doors
ui-door-row = The { $family } family, number { $number }
ui-door-row-vacant = Number { $number }, empty
ui-door-knock = Knock
ui-door-knocked = Knocked
ui-door-poster = A poster in the window
ui-door-last-warm = { $month }: it went well
ui-door-last-cool = { $month }: a cool reception
ui-door-last-not_in = { $month }: nobody in
ui-door-last-worry = about { $worry }
ui-door-more = Every door on these streets
ui-door-listen = Listen
ui-door-case = Make the case on…
ui-door-ask-vote = Ask for their vote
ui-door-ask-poster = Ask for a poster in the window
ui-door-move-on = Move on
ui-door-starts = Knocking starts a canvass of these streets: { $slots } slots, up to { $doors } doors.
ui-door-under-way = Canvassing these streets: { $left ->
        [0] the last door.
        [one] one more door after this.
       *[other] { $left } more doors after this.
    }
ui-door-next = Next door
ui-door-finish = Finish the street
ui-door-street-done = That is the street done.
ui-door-close = Close
ui-door-title = Number { $number }
ui-door-hint = Knock at number { $number }
ui-doors-blocked-away = Go to these streets to knock.
ui-doors-blocked-time = Not enough time left this turn to canvass.
ui-doors-canvass-all = Canvass the street
