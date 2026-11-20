## Knowledge is earned (DEC-091): what views show where the player does not
## know something yet, the knowledge levels, the notebook's titles and how
## things were learned. Arguments named `…_key` are Fluent keys.

# Levels.
knowledge-level-unknown = Not known yet
knowledge-level-heard = Heard of
knowledge-level-known = Known
knowledge-level-detailed = Known in detail
knowledge-as-of = as of { $month }/{ $year }
knowledge-stale = old figures
knowledge-rumoured = rumoured
knowledge-lead = Lead

# Placeholders (unknown → the placeholder, and the lead where there is one).
knowledge-broker-unknown = Someone runs things here — ask around.
knowledge-priorities-unknown = Not known yet — ask around.
knowledge-concerns-unknown = What worries people here isn't known yet — ask around.
knowledge-person-unnamed = Someone you haven't met
knowledge-stances-unknown = Stances not known yet — talk to them.
knowledge-character-unknown = You know little about them yet. Talk to them to learn more.
knowledge-support-unknown = Who supports it isn't known yet — ask around.
knowledge-proposal-stub = You've only heard of this. Find out more before drafting it.
knowledge-goals = Wants: { $list }
knowledge-favours-with = Owes or is owed by: { $list }

# Support in a word (from the overall figure; always shown).
support-band-popular = Popular
support-band-divisive = Divisive
support-band-unpopular = Unpopular

# What the rival is planning ($rival: their name).
rival-plan-something = { $rival } is planning something.
rival-plan-seat = { $rival } means to run for { $seat }.
rival-plan-focus = { $rival } means to run for { $seat }, campaigning on { $focus }.
rival-seat-mayor = the mayoralty
rival-seat-ward = your seat in ward { $ward }

# The notebook: sections and titles.
notebook-section-people = People
notebook-section-neighbourhoods = Neighbourhoods
notebook-section-power = Who runs things
notebook-section-proposals = Proposals
notebook-section-places = Places
notebook-section-rival = The rival
notebook-section-support = Support for policies
notebook-section-secrets = Gossip
notebook-town-concerns = What { $town } worries about
notebook-hood-concerns = What { $hood } worries about
notebook-broker = Who really runs { $town }
notebook-person = { $name }
notebook-rival-plans = What the rival is planning
notebook-policy-support = Who supports { $policy }
notebook-proposal = A proposal
notebook-situational = { $policy }
notebook-place = { $place }
notebook-layer = A map of { $town }
notebook-secret = Gossip about someone

# How something was learned.
learned-start = What you knew from the start
learned-local-option = Local knowledge
learned-primer = A party contact's briefing
learned-migrated = What you already knew
learned-visit = A visit: { $kind }
learned-overheard = Overheard: { $kind }
learned-action = { $action }
learned-met = Meeting { $person }
learned-service = { $service }
learned-story = The paper
learned-decision = A decision you faced
learned-council = Council business
learned-sponsor = A sponsor's briefing
learned-talk = Talking with { $person }
learned-lab = The lab

# Places with purpose (PL1): what a visit teaches.
notebook-sector = { $sector } in { $town }: { $outlook ->
        [growing] taking people on
        [shedding] letting people go
        [steady] steady
       *[other] how it is doing
    }
notebook-site = Conditions at { $place }{ $pollution ->
        [high] : heavy pollution
        [some] : some pollution
        [low] : clean air
       *[other] {""}
    }{ $hazard ->
        [high] , badly exposed to floods or fire
        [some] , some risk of floods or fire
       *[other] {""}
    }
notebook-service = What { $place } needs{ $need ->
        [high] : badly stretched
        [some] : stretched in places
        [low] : coping
       *[other] {""}
    }
