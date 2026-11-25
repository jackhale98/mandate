## What a bill does (DEC-111): the estimate before a vote. The engine names
## every key here and gives the arguments; sizes ($lo, $hi) arrive already
## formatted and are never signed. Each branch is a whole sentence. The
## horizon is a heading above the lines, never part of one.

impact-heading = What it does
impact-horizon-y1 = After a year
impact-horizon-y3 = After three years
impact-horizon-y5 = After five years
impact-in-time = in time for the election

## Direct counts: the bill's own arithmetic.

impact-target-homes-started = { $lo } homes started.
impact-target-extra-building = Private builders may start { $lo }% more homes.
impact-target-jobs-supported = { $lo } jobs supported in { $sector }.
impact-target-jobs-created = { $lo } jobs created in { $sector }.
impact-target-paid = { $class ->
    [low] Pays { $lo } a year to each adult on a low income.
    [middle] Pays { $lo } a year to each adult on a middle income.
   *[high] Pays { $lo } a year to each adult on a high income.
  }
impact-target-wage-uplift = Low pay rises { $lo }%.
impact-target-service-level = Service in { $category } rises { $lo }%.
impact-target-tax-points = { $direction ->
    [down] The property tax falls by { $lo } points of a home's value.
   *[up] The property tax rises by { $lo } points of a home's value.
  }
impact-target-rent-cap = Rents may rise at most { $lo }% a year.
impact-target-protection = Damage from a { $hazard } is cut by { $lo }%.
impact-target-labour-supply = { $direction ->
    [down] { $class ->
        [low] { $lo }% fewer people on low incomes look for work.
        [middle] { $lo }% fewer people on middle incomes look for work.
       *[high] { $lo }% fewer people on high incomes look for work.
      }
   *[up] { $class ->
        [low] { $lo }% more people on low incomes look for work.
        [middle] { $lo }% more people on middle incomes look for work.
       *[high] { $lo }% more people on high incomes look for work.
      }
  }

impact-hazard-flood = flood
impact-hazard-storm = storm
impact-hazard-wildfire = wildfire

impact-category-administration = the town hall's own work
impact-category-education = schools
impact-category-health = health care
impact-category-safety = policing and safety
impact-category-infrastructure = roads and transit
impact-category-social = social support
impact-category-housing = housing
impact-category-environment = parks and the environment
impact-category-economic_development = business support

## Outcomes: what reaches the place, against the same place without the law.

impact-homes-up = { $form ->
    [about] About { $lo } more homes than there would otherwise be.
    [range] Between { $lo } and { $hi } more homes than there would otherwise be.
   *[words] More homes than there would otherwise be.
  }
impact-homes-down = { $form ->
    [about] About { $lo } fewer homes than there would otherwise be.
    [range] Between { $lo } and { $hi } fewer homes than there would otherwise be.
   *[words] Fewer homes than there would otherwise be.
  }
impact-homes-flat = No clear change in the number of homes.

impact-jobs-up = { $form ->
    [about] About { $lo } more jobs overall.
    [range] Between { $lo } and { $hi } more jobs overall.
   *[words] More jobs overall.
  }
impact-jobs-down = { $form ->
    [about] About { $lo } fewer jobs overall.
    [range] Between { $lo } and { $hi } fewer jobs overall.
   *[words] Fewer jobs overall.
  }
impact-jobs-flat = No clear change in jobs overall.

impact-sector-jobs-up = { $form ->
    [about] About { $lo } more jobs in { $sector }.
    [range] Between { $lo } and { $hi } more jobs in { $sector }.
   *[words] More jobs in { $sector }.
  }
impact-sector-jobs-down = { $form ->
    [about] About { $lo } fewer jobs in { $sector }.
    [range] Between { $lo } and { $hi } fewer jobs in { $sector }.
   *[words] Fewer jobs in { $sector }.
  }
impact-sector-jobs-flat = No clear change in jobs in { $sector }.

impact-rent-up = { $form ->
    [about] Rents end about { $lo }% higher.
    [range] Rents end { $lo }% to { $hi }% higher.
   *[words] Rents end higher.
  }
impact-rent-down = { $form ->
    [about] Rents end about { $lo }% lower.
    [range] Rents end { $lo }% to { $hi }% lower.
   *[words] Rents end lower.
  }
impact-rent-flat = No clear change in rents.

impact-low-income-up = { $form ->
    [about] Incomes on low pay end about { $lo }% higher.
    [range] Incomes on low pay end { $lo }% to { $hi }% higher.
   *[words] Incomes on low pay end higher.
  }
impact-low-income-down = { $form ->
    [about] Incomes on low pay end about { $lo }% lower.
    [range] Incomes on low pay end { $lo }% to { $hi }% lower.
   *[words] Incomes on low pay end lower.
  }
impact-low-income-flat = No clear change in incomes on low pay.

impact-debt-up = { $form ->
    [about] Adds about { $lo } to the debt.
    [range] Adds between { $lo } and { $hi } to the debt.
   *[words] Adds to the debt.
  }
impact-debt-down = { $form ->
    [about] Takes about { $lo } off the debt.
    [range] Takes between { $lo } and { $hi } off the debt.
   *[words] Takes some off the debt.
  }
impact-debt-flat = No clear change in the debt.

impact-services-up = { $form ->
    [about] Services overall run about { $lo }% better.
    [range] Services overall run { $lo }% to { $hi }% better.
   *[words] Services overall run better.
  }
impact-services-down = { $form ->
    [about] Services overall run about { $lo }% worse.
    [range] Services overall run { $lo }% to { $hi }% worse.
   *[words] Services overall run worse.
  }
impact-services-flat = No clear change in services overall.

impact-approval-up = { $group ->
    [low] { $form ->
        [about] Approval among people on low incomes ends about { $lo } points higher.
        [range] Approval among people on low incomes ends { $lo } to { $hi } points higher.
       *[words] Approval among people on low incomes ends higher.
      }
    [middle] { $form ->
        [about] Approval among people on middle incomes ends about { $lo } points higher.
        [range] Approval among people on middle incomes ends { $lo } to { $hi } points higher.
       *[words] Approval among people on middle incomes ends higher.
      }
    [high] { $form ->
        [about] Approval among people on high incomes ends about { $lo } points higher.
        [range] Approval among people on high incomes ends { $lo } to { $hi } points higher.
       *[words] Approval among people on high incomes ends higher.
      }
   *[all] { $form ->
        [about] Approval of the government ends about { $lo } points higher.
        [range] Approval of the government ends { $lo } to { $hi } points higher.
       *[words] Approval of the government ends higher.
      }
  }
impact-approval-down = { $group ->
    [low] { $form ->
        [about] Approval among people on low incomes ends about { $lo } points lower.
        [range] Approval among people on low incomes ends { $lo } to { $hi } points lower.
       *[words] Approval among people on low incomes ends lower.
      }
    [middle] { $form ->
        [about] Approval among people on middle incomes ends about { $lo } points lower.
        [range] Approval among people on middle incomes ends { $lo } to { $hi } points lower.
       *[words] Approval among people on middle incomes ends lower.
      }
    [high] { $form ->
        [about] Approval among people on high incomes ends about { $lo } points lower.
        [range] Approval among people on high incomes ends { $lo } to { $hi } points lower.
       *[words] Approval among people on high incomes ends lower.
      }
   *[all] { $form ->
        [about] Approval of the government ends about { $lo } points lower.
        [range] Approval of the government ends { $lo } to { $hi } points lower.
       *[words] Approval of the government ends lower.
      }
  }
impact-approval-flat = { $group ->
    [low] No clear change in approval among people on low incomes.
    [middle] No clear change in approval among people on middle incomes.
    [high] No clear change in approval among people on high incomes.
   *[all] No clear change in approval of the government.
  }

## The money: exact.

impact-cost = Costs { $amount } a year, { $share }% of the budget.
impact-cost-none = Costs nothing to run.
impact-paid-rates = Paid from the budget as it stands.
impact-paid-borrowing = { $borrowed }% of the cost is borrowed.
impact-paid-levy = The tax it brings pays for it.
impact-paid-squeeze = More than the budget can carry: it and every other programme would be funded at { $funded }%.

## Who gains and who loses.

impact-gains-low = People on low incomes gain.
impact-gains-middle = People on middle incomes gain.
impact-gains-high = People on high incomes gain.
impact-gains-older = Older residents gain.
impact-gains-area = The streets it touches are spared disruption.
impact-loses-low = People on low incomes lose.
impact-loses-middle = People on middle incomes lose.
impact-loses-high = People on high incomes lose.
impact-loses-older = Older residents lose.
impact-loses-area = Streets are dug up while it builds.

## When, and how sure.

impact-when-at-once = In force from the day it passes.
impact-when-first-year = Funded at { $funded }% until the next budget.
impact-when-building = { $n ->
    [one] Work starts within a month.
   *[other] Work starts within { $n } months.
  }
impact-when-full = { $n ->
    [one] In full effect after a month.
   *[other] In full effect after { $n } months.
  }
impact-sure-rough = Rough estimate.
impact-sure-fair = Fair estimate.
impact-sure-firm = Firm figure.
impact-source-statistician = The statistician's figures.
