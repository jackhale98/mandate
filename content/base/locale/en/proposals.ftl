## Policy-family proposals (DEC-092): why a proposal cannot be drafted, why
## an option cannot be chosen, and what a sponsor's version costs. The engine
## names every key here and gives the arguments.

proposal-blocked-lapsed = This proposal is no longer on the table.
proposal-blocked-heard = You've only heard of this. Find out more before drafting it.
proposal-blocked-no-power = Your post cannot propose this here.

option-unavailable = This choice cannot be made here.
option-unavailable-needs = This needs powers this place does not have.
option-unavailable-excludes = This cannot go with { $with }.

sponsor-price-favour = { $count ->
    [1] You would owe them a favour for { $context }.
   *[other] You would owe them { $count } favours for { $context }.
  }
sponsor-price-endorsement = { $who ->
    [area] You would stand with them in public: { $area } would think better of you, and some others less.
    [classes] { $classes ->
        [low] You would stand with them in public: people on low incomes would think better of you, and some others less.
        [middle] You would stand with them in public: people on middle incomes would think better of you, and some others less.
        [high] You would stand with them in public: people on high incomes would think better of you, and some others less.
       *[other] You would stand with them in public: their supporters would think better of you, and some others less.
      }
   *[everyone] You would stand with them in public: some would think better of you, and some less.
  }
