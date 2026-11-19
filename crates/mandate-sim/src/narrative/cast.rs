//! Casting roles from live actors and evaluating conditions on live state in
//! the player's town (DEC-032, DEC-077 §4). Read-only: the same functions
//! serve the director (inside the scheduler), NPC goals (in the politics
//! domain), the Commands phase, views, and the grounding audit.

use super::{Actor, CastEntry, Reason};
use crate::characters::{self, CharacterId};
use crate::player::PlayerState;
use crate::policy::{PolicyInstance, PolicyStatus};
use crate::politics::{PoliticianId, PoliticsState, Role};
use crate::scheduler::Reads;
use mandate_content::{
    BillSponsor, Cast, Condition, ContentBundle, OfficeRole, PartyRel, Power, RoleDef, TrackKey,
};
use mandate_core::SimDate;

/// Read-only access for casting and conditions. `pol` is passed separately
/// because the politics domain evaluates goals while it is being written.
pub struct View<'a> {
    pub r: &'a Reads<'a>,
    pub pol: &'a PoliticsState,
    /// People spotlighted lately: cast others first, for variety (order
    /// only; never who may be cast).
    pub recent: Vec<CharacterId>,
}

impl<'a> View<'a> {
    pub fn new(r: &'a Reads<'a>, pol: &'a PoliticsState) -> Self {
        View {
            r,
            pol,
            recent: Vec::new(),
        }
    }

    pub fn content(&self) -> &'a ContentBundle {
        self.r.content
    }

    pub fn date(&self) -> SimDate {
        self.r.date
    }

    pub fn player(&self) -> Option<&'a PlayerState> {
        self.r.player
    }

    /// The player's town (municipality index).
    pub fn town(&self) -> Option<u32> {
        self.player().map(|p| p.municipality.0)
    }

    /// The player's character.
    pub fn me(&self) -> Option<CharacterId> {
        let pl = self.player()?;
        self.pol.get(pl.politician).map(|p| p.character)
    }

    pub fn town_character(&self) -> Option<&'a characters::TownCharacter> {
        self.pol.characters.towns.get(self.town()? as usize)
    }

    /// When play began: the end of the history warm-up (DEC-022).
    pub fn play_began(&self) -> SimDate {
        self.r.meta.start_date.add_days(
            self.content().tuning.burn_in_years as u32 * mandate_core::time::DAYS_PER_YEAR,
        )
    }

    /// The municipality a character belongs to.
    pub fn town_of(&self, id: CharacterId) -> Option<u32> {
        characters::municipality_of(self.pol, self.r.world, id)
    }

    /// The player's next local election day (a running race's, else the calendar's).
    pub fn next_election(&self) -> Option<SimDate> {
        let pl = self.player()?;
        let me = pl.politician;
        if let Some(r) = self
            .pol
            .elections
            .races
            .iter()
            .find(|r| r.candidates.iter().any(|c| c.politician == me))
        {
            return Some(r.election_day);
        }
        if let Some(d) = self.pol.elections.next_day.filter(|d| *d >= self.date()) {
            return Some(d);
        }
        let (cal, term) = crate::politics::elections::local_calendar(self.r)?;
        Some(crate::politics::elections::next_election_day(
            cal,
            term,
            self.date(),
        ))
    }

    /// The player is standing in a running race.
    pub fn campaigning(&self) -> bool {
        let Some(pl) = self.player() else {
            return false;
        };
        characters::is_candidate(self.pol, pl.politician)
    }

    fn policy(&self, id: u32) -> Option<&'a PolicyInstance> {
        self.r.policy().instances.get(id as usize)
    }

    /// What the player's party expects on a bill: +1 yes, −1 no, 0 free vote.
    pub fn party_line(&self, bill: u32) -> i8 {
        let Some(pl) = self.player() else { return 0 };
        let mine = self.pol.get(pl.politician).and_then(|p| p.party);
        let sponsor = self
            .policy(bill)
            .and_then(|p| p.sponsor)
            .and_then(|s| self.pol.get(s))
            .and_then(|s| s.party);
        match (mine, sponsor) {
            (Some(a), Some(b)) if a == b => 1,
            (Some(_), Some(_)) => -1,
            _ => 0,
        }
    }

    /// How the player's ward leans on a bill: its public support there
    /// (positive = for).
    pub fn ward_lean(&self, bill: u32) -> f64 {
        let (Some(pl), Some(inst)) = (self.player(), self.policy(bill)) else {
            return 0.0;
        };
        let Some(t) = self.content().policies.get(&inst.template) else {
            return 0.0;
        };
        let hoods = crate::policy::ward_hoods(self.r.world, pl.ward);
        crate::policy::public_support(t, self.r.population(), &hoods)
    }

    /// The player can still declare a vote on the bill.
    pub fn can_vote(&self, bill: u32) -> bool {
        let Some(pl) = self.player() else {
            return false;
        };
        let Some(p) = self.policy(bill) else {
            return false;
        };
        if p.status != PolicyStatus::UnderConsideration
            || p.jurisdiction != pl.municipality.0
            || p.sponsor == Some(pl.politician)
            || self
                .pol
                .declared_votes
                .contains_key(&(pl.politician.0, bill))
        {
            return false;
        }
        let Some(me) = self.pol.get(pl.politician) else {
            return false;
        };
        let powers = crate::actions::powers_for(self.content(), &self.r.meta.constitution, me.role);
        powers.contains(&Power::ApproveOrdinance) || powers.contains(&Power::Veto)
    }

    fn track(&self, t: TrackKey) -> Option<f64> {
        let pl = self.player()?;
        Some(match t {
            TrackKey::Viability => pl.tracks.viability as f64,
            TrackKey::LegalJeopardy => pl.tracks.legal_jeopardy as f64,
            TrackKey::Threat => pl.tracks.threat as f64,
            TrackKey::Standing => pl.tracks.standing as f64,
            TrackKey::Health => pl.tracks.health as f64,
            TrackKey::Mandate => pl.mandate as f64,
            TrackKey::Record => self.pol.get(pl.politician).map(|p| p.record as f64)? * 100.0,
        })
    }

    /// Named people of the town with an occupation.
    fn named_in_town(&self, occupations: &[String]) -> Vec<CharacterId> {
        let Some(m) = self.town() else {
            return Vec::new();
        };
        self.pol
            .characters
            .people
            .iter()
            .filter(|c| {
                c.named
                    .as_deref()
                    .is_some_and(|n| occupations.contains(&n.occupation))
            })
            .filter(|c| self.town_of(c.id) == Some(m))
            .map(|c| c.id)
            .collect()
    }

    /// Council members and the mayor of the town (not the player).
    pub fn officeholders(&self) -> Vec<PoliticianId> {
        let Some(m) = self.town() else {
            return Vec::new();
        };
        let Some(c) = self.pol.councils.get(m as usize) else {
            return Vec::new();
        };
        c.members
            .iter()
            .chain(c.mayor.iter())
            .copied()
            .filter(|id| self.pol.get(*id).is_some_and(|p| !p.is_player))
            .collect()
    }
}

fn in_range(v: f64, min: Option<f64>, max: Option<f64>) -> bool {
    min.is_none_or(|m| v >= m) && max.is_none_or(|m| v <= m)
}

fn occupation_rank(
    v: &View<'_>,
    id: CharacterId,
    list: &[String],
) -> (u8, u8, u8, i64, usize, u32) {
    let content = v.content();
    let crisis = v
        .town_character()
        .and_then(|t| t.crisis())
        .map(|c| c.kind.as_str());
    let n = v.pol.characters.get(id).and_then(|c| c.named.as_deref());
    let occ = n.and_then(|n| content.occupation(&n.occupation));
    let stake = occ.is_some_and(|o| crisis.is_some_and(|c| o.stake.crises.iter().any(|x| x == c)));
    let leader = occ.is_some_and(|o| o.leader);
    let influence = occ.map(|o| (o.influence * 1000.0) as i64).unwrap_or(0);
    let order = n
        .and_then(|n| list.iter().position(|o| *o == n.occupation))
        .unwrap_or(usize::MAX);
    // Sort ascending: stake first, then someone not seen lately, leaders,
    // influence, list order, id.
    let seen = v.recent.contains(&id);
    (
        !stake as u8,
        seen as u8,
        !leader as u8,
        -influence,
        order,
        id.0,
    )
}

/// Every actor that could fill a role, best first (deterministic).
pub fn candidates(
    v: &View<'_>,
    cast: &Cast,
    so_far: &[CastEntry],
    goal_holder: Option<CharacterId>,
) -> Vec<Actor> {
    let me = v.me();
    let Some(m) = v.town() else {
        return Vec::new();
    };
    let person = |id: CharacterId| Actor::Person { id };
    let not_me = |id: &CharacterId| Some(*id) != me;
    match cast {
        Cast::Broker => v
            .town_character()
            .and_then(|t| t.broker)
            .filter(not_me)
            .map(person)
            .into_iter()
            .collect(),
        Cast::Rival => {
            // The town's rival; in a campaign, whoever stands against the
            // player (the town's rival first if they do).
            let named = v.town_character().and_then(|t| t.rival).filter(not_me);
            let mut out: Vec<Actor> = Vec::new();
            if v.campaigning() {
                let me_pol = v.player().map(|p| p.politician);
                let opponents: Vec<CharacterId> = v
                    .pol
                    .elections
                    .races
                    .iter()
                    .filter(|r| r.candidates.iter().any(|c| Some(c.politician) == me_pol))
                    .flat_map(|r| r.candidates.iter())
                    .filter(|c| Some(c.politician) != me_pol)
                    .filter_map(|c| v.pol.get(c.politician).map(|p| p.character))
                    .collect();
                if let Some(r) = named.filter(|r| opponents.contains(r)) {
                    out.push(person(r));
                }
                out.extend(
                    opponents
                        .into_iter()
                        .filter(|o| Some(*o) != named)
                        .map(person),
                );
            } else if let Some(r) = named {
                out.push(person(r));
            }
            out
        }
        Cast::Mayor => v
            .pol
            .councils
            .get(m as usize)
            .and_then(|c| c.mayor)
            .and_then(|p| v.pol.get(p))
            .filter(|p| !p.is_player)
            .map(|p| person(p.character))
            .into_iter()
            .collect(),
        Cast::Occupation { occupations } => {
            let mut ids = v.named_in_town(occupations);
            ids.sort_by_key(|id| occupation_rank(v, *id, occupations));
            ids.into_iter().map(person).collect()
        }
        Cast::Colleague { party } => {
            let my_party = v
                .player()
                .and_then(|pl| v.pol.get(pl.politician))
                .and_then(|p| p.party);
            v.officeholders()
                .into_iter()
                .filter_map(|id| v.pol.get(id))
                .filter(|p| matches!(p.role, Role::Councillor { .. }))
                .filter(|p| match party {
                    PartyRel::Any => true,
                    PartyRel::Same => p.party.is_some() && p.party == my_party,
                    PartyRel::Other => p.party != my_party,
                })
                .map(|p| person(p.character))
                .collect()
        }
        Cast::Creditor => {
            let Some(me) = me else { return Vec::new() };
            let mut v2: Vec<(u32, SimDate, CharacterId)> = Vec::new();
            for f in v.pol.characters.favours.iter().filter(|f| f.debtor == me) {
                if v.town_of(f.creditor) != Some(m) {
                    continue;
                }
                match v2.iter_mut().find(|x| x.2 == f.creditor) {
                    Some(x) => {
                        x.0 += f.count as u32;
                        x.1 = x.1.min(f.since);
                    }
                    None => v2.push((f.count as u32, f.since, f.creditor)),
                }
            }
            v2.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
            v2.into_iter().map(|x| person(x.2)).collect()
        }
        Cast::Bill {
            sponsor,
            sponsored_by,
        } => {
            let player_pol = v.player().map(|p| p.politician);
            let by = sponsored_by
                .as_deref()
                .and_then(|r| super::person_of(so_far, r));
            let mut bills: Vec<&PolicyInstance> =
                v.r.policy()
                    .in_municipality(m)
                    .filter(|p| p.status == PolicyStatus::UnderConsideration)
                    .filter(|p| match sponsor {
                        BillSponsor::Any => true,
                        BillSponsor::Player => p.sponsor.is_some() && p.sponsor == player_pol,
                        BillSponsor::Other => p.sponsor != player_pol,
                    })
                    .filter(|p| {
                        sponsored_by.is_none()
                            || by.is_some_and(|c| {
                                p.sponsor
                                    .and_then(|s| v.pol.get(s))
                                    .is_some_and(|s| s.character == c)
                            })
                    })
                    .collect();
            bills.sort_by_key(|p| (p.next_stage_on, p.id.0));
            bills
                .into_iter()
                .map(|p| Actor::Policy { id: p.id.0 })
                .collect()
        }
        Cast::SponsorOf { role } => match super::actor_of(so_far, role) {
            Some(Actor::Policy { id }) => v
                .policy(id)
                .and_then(|p| p.sponsor)
                .and_then(|s| v.pol.get(s))
                .filter(|s| !s.is_player)
                .map(|s| person(s.character))
                .into_iter()
                .collect(),
            _ => Vec::new(),
        },
        Cast::WardNeighborhood => {
            let Some(pl) = v.player() else {
                return Vec::new();
            };
            let pop = v.r.population();
            let mut hoods: Vec<(i64, usize)> = crate::policy::ward_hoods(v.r.world, pl.ward)
                .into_iter()
                .map(|h| {
                    let cs = &pop.hoods[h].cohorts;
                    let n: f64 = cs.iter().map(|c| c.count as f64).sum::<f64>().max(1.0);
                    let gov: f64 = cs
                        .iter()
                        .map(|c| c.gov_approval as f64 * c.count as f64)
                        .sum::<f64>()
                        / n;
                    ((gov * 1e6) as i64, h)
                })
                .collect();
            hoods.sort();
            hoods
                .into_iter()
                .map(|(_, h)| Actor::Neighborhood { id: h as u32 })
                .collect()
        }
        Cast::Law {
            templates,
            building,
        } => {
            let mut laws: Vec<&PolicyInstance> =
                v.r.policy()
                    .in_municipality(m)
                    .filter(|p| matches!(p.status, PolicyStatus::Enacted | PolicyStatus::InEffect))
                    .filter(|p| templates.contains(&p.template))
                    .filter(|p| !building || p.progress < 1.0)
                    .collect();
            // Being built first, then the newest.
            laws.sort_by_key(|p| {
                (
                    (p.progress >= 1.0) as u8,
                    std::cmp::Reverse(p.enacted_on),
                    p.id.0,
                )
            });
            laws.into_iter()
                .map(|p| Actor::Policy { id: p.id.0 })
                .collect()
        }
        Cast::GoalHolder => goal_holder.filter(not_me).map(person).into_iter().collect(),
        Cast::Ally => {
            // Colleagues by their opinion of the player, warmest first.
            let mut xs: Vec<(f32, CharacterId)> = v
                .officeholders()
                .into_iter()
                .filter_map(|id| v.pol.get(id))
                .filter(|p| !p.is_player)
                .map(|p| {
                    let o = me
                        .map(|me| characters::opinion(v.pol, p.character, me))
                        .unwrap_or(0.0);
                    (o, p.character)
                })
                .collect();
            xs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            xs.into_iter().map(|(_, c)| person(c)).collect()
        }
    }
}

/// Cast every role (keeping `preset` entries by role id). People are not
/// cast twice in one decision. `None` if a role cannot be filled.
pub fn cast_roles(
    v: &View<'_>,
    roles: &[RoleDef],
    preset: &[CastEntry],
    goal_holder: Option<CharacterId>,
) -> Option<Vec<CastEntry>> {
    let mut out: Vec<CastEntry> = Vec::new();
    for r in roles {
        if let Some(p) = preset.iter().find(|p| p.role == r.id) {
            out.push(p.clone());
            continue;
        }
        let pick = candidates(v, &r.cast, &out, goal_holder)
            .into_iter()
            .find(|a| !out.iter().any(|c| c.actor == *a))?;
        out.push(CastEntry {
            role: r.id.clone(),
            actor: pick,
        });
    }
    // Storyline casts may carry roles the template does not name; keep them.
    for p in preset {
        if !out.iter().any(|c| c.role == p.role) {
            out.push(p.clone());
        }
    }
    Some(out)
}

/// Cast every role so that `ok` holds for the whole cast: a depth-first
/// search over each role's best candidates (at most `width` per role), in
/// order, so the result is deterministic. `None` if no cast satisfies it.
pub fn cast_search(
    v: &View<'_>,
    roles: &[RoleDef],
    preset: &[CastEntry],
    goal_holder: Option<CharacterId>,
    width: usize,
    ok: &dyn Fn(&[CastEntry]) -> bool,
) -> Option<Vec<CastEntry>> {
    #[allow(clippy::too_many_arguments)]
    fn go(
        v: &View<'_>,
        roles: &[RoleDef],
        preset: &[CastEntry],
        goal_holder: Option<CharacterId>,
        width: usize,
        ok: &dyn Fn(&[CastEntry]) -> bool,
        so_far: &mut Vec<CastEntry>,
        budget: &mut u32,
    ) -> bool {
        let Some(r) = roles.get(so_far.len()) else {
            let mut full = so_far.clone();
            for p in preset {
                if !full.iter().any(|c| c.role == p.role) {
                    full.push(p.clone());
                }
            }
            if ok(&full) {
                *so_far = full;
                return true;
            }
            return false;
        };
        if *budget == 0 {
            return false;
        }
        *budget -= 1;
        let options: Vec<Actor> = match preset.iter().find(|p| p.role == r.id) {
            Some(p) => vec![p.actor],
            None => candidates(v, &r.cast, so_far, goal_holder)
                .into_iter()
                .filter(|a| !so_far.iter().any(|c| c.actor == *a))
                .take(width)
                .collect(),
        };
        let depth = so_far.len();
        for a in options {
            so_far.push(CastEntry {
                role: r.id.clone(),
                actor: a,
            });
            if go(v, roles, preset, goal_holder, width, ok, so_far, budget) {
                return true;
            }
            so_far.truncate(depth);
        }
        false
    }
    let mut so_far = Vec::new();
    let mut budget = 200;
    go(
        v,
        roles,
        preset,
        goal_holder,
        width,
        ok,
        &mut so_far,
        &mut budget,
    )
    .then_some(so_far)
}

/// Does an actor still fill its role (the grounding audit, and lapsing)?
pub fn still_cast(
    v: &View<'_>,
    role: &RoleDef,
    actor: Actor,
    cast: &[CastEntry],
    goal_holder: Option<CharacterId>,
) -> bool {
    // A rival stays a rival while they stand against the player, even when
    // the polls make someone else the strongest opponent this week.
    if matches!(role.cast, Cast::Rival)
        && let Actor::Person { id } = actor
        && opposes_in_race(v, id)
    {
        return true;
    }
    candidates(v, &role.cast, cast, goal_holder).contains(&actor)
}

/// The person is a candidate in the player's running race.
pub fn opposes_in_race(v: &View<'_>, id: CharacterId) -> bool {
    let Some(pl) = v.player() else {
        return false;
    };
    v.pol.elections.races.iter().any(|r| {
        r.candidates.iter().any(|c| c.politician == pl.politician)
            && r.candidates.iter().any(|c| {
                c.politician != pl.politician
                    && v.pol.get(c.politician).is_some_and(|p| p.character == id)
            })
    })
}

/// Does a condition hold for this cast?
pub fn holds(v: &View<'_>, c: &Condition, cast: &[CastEntry]) -> bool {
    let me = v.me();
    let who = |role: &str| super::person_of(cast, role);
    let town = v.town_character();
    match c {
        Condition::Crisis { kind, min_severity } => town.is_some_and(|t| {
            t.crises
                .iter()
                .any(|x| &x.kind == kind && x.severity as f64 >= *min_severity)
        }),
        Condition::LiveCrisis { min_severity } => town
            .and_then(|t| t.crisis())
            .is_some_and(|x| x.severity as f64 >= *min_severity),
        Condition::Indicator {
            indicator,
            min,
            max,
        } => v.town().is_some_and(|m| {
            in_range(
                crate::concerns::indicator(v.r, m as usize, *indicator),
                *min,
                *max,
            )
        }),
        Condition::Concern { concern, top } => town.is_some_and(|t| {
            t.priorities
                .iter()
                .take(*top as usize)
                .any(|p| p == concern)
        }),
        Condition::Industry { drivers } => town
            .and_then(|t| t.industry.as_deref())
            .and_then(|s| v.content().sector(s))
            .is_some_and(|s| drivers.contains(&s.driver)),
        Condition::ElectionWithin { days } => v
            .next_election()
            .is_some_and(|d| v.date().days_until(d) <= *days && d >= v.date()),
        Condition::Campaigning { is } => v.campaigning() == *is,
        Condition::Office { role } => {
            let r = v
                .player()
                .and_then(|pl| v.pol.get(pl.politician))
                .map(|p| p.role);
            matches!(
                (role, r),
                (OfficeRole::Councillor, Some(Role::Councillor { .. }))
                    | (OfficeRole::Mayor, Some(Role::Mayor { .. }))
                    | (
                        OfficeRole::Any,
                        Some(Role::Councillor { .. } | Role::Mayor { .. })
                    )
                    | (OfficeRole::None, Some(Role::Private { .. }))
            )
        }
        Condition::Track { track, min, max } => {
            v.track(*track).is_some_and(|x| in_range(x, *min, *max))
        }
        Condition::Money { min, max } => v
            .player()
            .is_some_and(|pl| in_range(pl.money.units() as f64, *min, *max)),
        Condition::Opinion { role, min, max } => match (who(role), me) {
            (Some(p), Some(me)) => in_range(characters::opinion(v.pol, p, me) as f64, *min, *max),
            _ => false,
        },
        Condition::Favour {
            role,
            owed_to_player,
        } => match (who(role), me) {
            (Some(p), Some(me)) => {
                if *owed_to_player {
                    characters::favours_owed(v.pol, p, me) > 0
                } else {
                    characters::favours_owed(v.pol, me, p) > 0
                }
            }
            _ => false,
        },
        Condition::Remembers { role, kinds } => match (who(role), me) {
            (Some(p), Some(me)) => v.pol.characters.get(p).is_some_and(|c| {
                c.memories
                    .iter()
                    .any(|m| m.actor == me && kinds.iter().any(|k| k == m.kind.key()))
            }),
            _ => false,
        },
        Condition::Personality {
            role,
            axis,
            min,
            max,
        } => who(role)
            .and_then(|p| characters::profile(v.pol, v.r.world, p))
            .is_some_and(|p| {
                let x = &p.personality;
                let val = match axis.as_str() {
                    "ambition" => x.ambition,
                    "integrity" => x.integrity,
                    "warmth" => x.warmth,
                    "boldness" => x.boldness,
                    _ => x.loyalty,
                };
                in_range(val as f64, *min, *max)
            }),
        Condition::Trait { role, traits } => who(role)
            .and_then(|p| v.pol.characters.get(p))
            .is_some_and(|c| c.traits.iter().any(|t| traits.contains(t))),
        Condition::LawInForce { templates, absent } => {
            let any = v.town().is_some_and(|m| {
                v.r.policy().in_municipality(m).any(|p| {
                    matches!(p.status, PolicyStatus::Enacted | PolicyStatus::InEffect)
                        && templates.contains(&p.template)
                })
            });
            any != *absent
        }
        Condition::PartyLineAgainstWard { role } => match super::actor_of(cast, role) {
            Some(Actor::Policy { id }) => {
                let line = v.party_line(id);
                let lean = v.ward_lean(id);
                line != 0 && lean.abs() > 0.02 && (line as f64) * lean < 0.0
            }
            _ => false,
        },
        Condition::CanVote { role } => match super::actor_of(cast, role) {
            Some(Actor::Policy { id }) => v.can_vote(id),
            _ => false,
        },
        Condition::Goal { role, goal } => who(role).is_some_and(|p| {
            v.pol
                .characters
                .goals
                .iter()
                .any(|g| g.holder == p && &g.goal == goal)
        }),
        Condition::MonthsInPlay { min } => {
            v.play_began().days_until(v.date()) >= min * 30 && v.date() >= v.play_began()
        }
        Condition::Knows { subject, min } => super::learn::knows(v, subject, *min, cast, None),
    }
}

/// Why the situation came up (for relevance and the "why" panel).
pub fn reasons(v: &View<'_>, conds: &[Condition], cast: &[CastEntry]) -> Vec<Reason> {
    let mut out = Vec::new();
    let town = v.town_character();
    for c in conds {
        match c {
            Condition::Crisis { kind, .. } => {
                if let Some(x) = town.and_then(|t| t.crises.iter().find(|x| &x.kind == kind)) {
                    out.push(Reason::Crisis {
                        kind: kind.clone(),
                        severity: x.severity,
                    });
                }
            }
            Condition::LiveCrisis { .. } => {
                if let Some(x) = town.and_then(|t| t.crisis()) {
                    out.push(Reason::Crisis {
                        kind: x.kind.clone(),
                        severity: x.severity,
                    });
                }
            }
            Condition::Concern { concern, .. } => out.push(Reason::Concern {
                concern: concern.clone(),
            }),
            Condition::ElectionWithin { .. } | Condition::Campaigning { is: true } => {
                if let Some(d) = v.next_election() {
                    out.push(Reason::Election {
                        days: v.date().days_until(d),
                    });
                }
            }
            Condition::Track { track, .. } => out.push(Reason::Pressure {
                track: format!("{track:?}").to_lowercase(),
            }),
            Condition::Goal { role, goal } => {
                if let Some(p) = super::person_of(cast, role) {
                    out.push(Reason::Goal {
                        person: p,
                        goal: goal.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    if let Some(me) = v.me() {
        for e in cast {
            if let Actor::Person { id } = e.actor
                && v.pol
                    .characters
                    .get(id)
                    .is_some_and(|c| c.memories.iter().any(|m| m.actor == me))
                && !out
                    .iter()
                    .any(|r| matches!(r, Reason::Memory { person } if *person == id))
            {
                out.push(Reason::Memory { person: id });
            }
        }
    }
    if cast
        .iter()
        .any(|e| matches!(e.actor, Actor::Neighborhood { .. }))
    {
        out.push(Reason::Ward);
    }
    out
}

/// Every cast actor still exists (the grounding audit's first test).
pub fn actors_exist(v: &View<'_>, cast: &[CastEntry]) -> bool {
    cast.iter().all(|e| match e.actor {
        Actor::Person { id } => v.pol.characters.get(id).is_some(),
        Actor::Neighborhood { id } => (id as usize) < v.r.world.neighborhoods.len(),
        Actor::Policy { id } => v.r.policy().instances.get(id as usize).is_some(),
    })
}
