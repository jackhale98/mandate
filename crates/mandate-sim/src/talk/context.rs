//! A conversation's context and the shared condition type (§3.3): the
//! slots (`speaker`, `about`, `player`; the place is `here`), what the
//! speaker will tell, and `SituationCondition` evaluated with the Fluent
//! arguments each condition provides. Town-level variants are the policy
//! domain's (`policy::situation_holds`); this adds the slot-relative ones.
//! Read-only.

use super::AboutRef;
use crate::characters::{self, CharacterId};
use crate::places::Location;
use crate::player::knowledge::{self, Level, PolicyRef, Subject};
use crate::state::State;
use crate::world::{MunicipalityId, NeighborhoodId};
use mandate_content::conversation::{HoodOf, SubjectRef};
use mandate_content::{ContentBundle, SituationCondition};
use std::collections::BTreeMap;

pub type Args = BTreeMap<String, String>;

/// Everything a conversation's conditions read.
pub struct Ctx<'a> {
    pub st: &'a State,
    pub content: &'a ContentBundle,
    /// The player's town.
    pub town: MunicipalityId,
    /// The player's character.
    pub me: CharacterId,
    pub speaker: CharacterId,
    pub about: Option<&'a AboutRef>,
    pub here: Location,
    /// The speaker's discretion cap (§3.4).
    pub cap: Level,
}

fn in_range(v: f64, min: Option<f64>, max: Option<f64>) -> bool {
    min.is_none_or(|a| v >= a) && max.is_none_or(|z| v <= z)
}

/// The fallback for an argument naming a Fluent key the facts lack.
const SUBJECT_FALLBACK: &str = "talk-subject-that";
const STAKE_FALLBACK: &str = "talk-stake-town";

impl Ctx<'_> {
    pub fn name_of(&self, c: CharacterId) -> String {
        characters::profile(&self.st.politics, &self.st.world, c)
            .map(|p| p.full_name())
            .unwrap_or_default()
    }

    fn town_character(&self) -> Option<&crate::characters::TownCharacter> {
        self.st.politics.characters.towns.get(self.town.index())
    }

    pub fn broker(&self) -> Option<CharacterId> {
        self.town_character().and_then(|t| t.broker)
    }

    pub fn rival(&self) -> Option<CharacterId> {
        self.town_character().and_then(|t| t.rival)
    }

    /// The person in a slot.
    pub fn person(&self, slot: &str) -> Option<CharacterId> {
        match slot {
            "speaker" => Some(self.speaker),
            "player" => Some(self.me),
            "about" => match self.about? {
                AboutRef::Person { person } => Some(*person),
                AboutRef::Rival => self.rival(),
                _ => None,
            },
            _ => None,
        }
    }

    /// A neighbourhood named from the conversation.
    pub fn hood(&self, of: HoodOf) -> Option<NeighborhoodId> {
        match of {
            HoodOf::Here => crate::places::hood_of(self.st, self.here),
            HoodOf::SpeakerHome => {
                characters::profile(&self.st.politics, &self.st.world, self.speaker)
                    .map(|p| NeighborhoodId(p.home))
            }
            HoodOf::About => match self.about? {
                AboutRef::Hood { hood } => Some(*hood),
                AboutRef::Place { place } => self
                    .st
                    .world
                    .places
                    .get(place.index())
                    .map(|p| p.neighborhood),
                _ => None,
            },
        }
    }

    /// The template of the policy subject (a bill's, or a template).
    fn about_template(&self) -> Option<String> {
        match self.about? {
            AboutRef::Bill { policy } => self
                .st
                .policy
                .instances
                .get(policy.index())
                .map(|p| p.template.clone()),
            AboutRef::Template { template } => Some(template.clone()),
            _ => None,
        }
    }

    /// The ledger subject a reference names here (`None`: it names nothing
    /// in this conversation, e.g. a town without a broker).
    pub fn subject(&self, r: &SubjectRef) -> Option<Subject> {
        let town = self.town;
        match r {
            SubjectRef::TownConcerns => Some(Subject::TownConcerns { town }),
            SubjectRef::HoodConcerns { of } => {
                let hood = self.hood(*of)?;
                let h = self.st.world.neighborhoods.get(hood.index())?;
                (h.municipality == town).then_some(Subject::HoodConcerns { hood })
            }
            SubjectRef::Broker => self.broker().map(|_| Subject::Broker { town }),
            SubjectRef::Person { slot } => {
                let person = self.person(slot)?;
                (person != self.me).then_some(Subject::Person { person })
            }
            SubjectRef::RivalPlans => self.rival().map(|_| Subject::RivalPlans { town }),
            // Policy families (F-series) bring proposals and their sponsors.
            SubjectRef::Proposal { .. } => None,
            SubjectRef::PolicySupport { .. } => {
                let template = self.about_template()?;
                Some(Subject::PolicySupport {
                    town,
                    policy: PolicyRef::Template { template },
                })
            }
        }
    }

    /// What the speaker would tell of a subject (§3.4).
    pub fn told(&self, s: &Subject) -> Level {
        characters::knows::tells(self.st, self.content, self.speaker, s, self.cap)
    }

    fn concern_key(&self, i: usize) -> Option<String> {
        self.content.concerns.get(i).map(|c| c.name_key.clone())
    }

    fn place_name_of(&self, c: CharacterId) -> String {
        let st = self.st;
        let named_place = st
            .politics
            .characters
            .get(c)
            .and_then(|x| x.named.as_deref())
            .and_then(|n| n.place)
            .and_then(|p| st.world.places.get(p.index()));
        let office = || {
            let kind = match st
                .politics
                .characters
                .get(c)
                .and_then(|x| x.politician)
                .and_then(|p| st.politics.get(p))
                .map(|p| p.role)
            {
                Some(crate::politics::Role::Mayor { .. }) => crate::world::PlaceKind::TownHall,
                _ => crate::world::PlaceKind::CouncilChamber,
            };
            st.world
                .places
                .iter()
                .find(|p| p.municipality == self.town && p.kind == kind)
        };
        named_place
            .or_else(office)
            .map(|p| p.name.clone())
            .or_else(|| {
                st.world
                    .municipalities
                    .get(self.town.index())
                    .map(|m| m.name.clone())
            })
            .unwrap_or_default()
    }

    /// The broker's arguments: `$broker`, `$broker_place`, `$stake_key`.
    fn broker_args(&self, b: CharacterId, args: &mut Args) {
        args.insert("broker".into(), self.name_of(b));
        args.insert("broker_place".into(), self.place_name_of(b));
        let stake = self
            .town_character()
            .and_then(|t| t.industry.as_deref())
            .and_then(|s| self.content.sector(s))
            .and_then(|s| s.talk_key.clone())
            .unwrap_or_else(|| STAKE_FALLBACK.into());
        args.insert("stake_key".into(), stake);
    }

    /// The arguments a subject provides (`SubjectRef::args`).
    fn subject_args(&self, s: &Subject, args: &mut Args) {
        let st = self.st;
        match s {
            Subject::TownConcerns { town } => {
                let top = crate::concerns::top_concerns(&st.population, town.index());
                if let Some(k) = top.first().and_then(|(i, _)| self.concern_key(*i)) {
                    args.insert("concern_key".into(), k);
                }
            }
            Subject::HoodConcerns { hood } => {
                if let Some(h) = st.world.neighborhoods.get(hood.index()) {
                    args.insert("hood".into(), h.name.clone());
                    let felt = crate::concerns::felt_concerns(
                        &st.population,
                        self.content,
                        h.municipality.index(),
                        &[hood.index()],
                    );
                    if let Some(k) = felt.first().and_then(|(i, _)| self.concern_key(*i)) {
                        args.insert("concern_key".into(), k);
                    }
                }
            }
            Subject::Broker { .. } => {
                if let Some(b) = self.broker() {
                    self.broker_args(b, args);
                }
            }
            Subject::Person { person } => {
                args.insert("person".into(), self.name_of(*person));
            }
            Subject::RivalPlans { .. } => {
                if let Some(r) = self.rival() {
                    args.insert("rival".into(), self.name_of(r));
                }
            }
            Subject::PolicySupport {
                town,
                policy: PolicyRef::Template { template },
            } => {
                if let Some(t) = self.content.policies.get(template) {
                    args.insert("policy_key".into(), t.name_key.clone());
                    let hoods: Vec<usize> = st.world.hoods_of(*town).collect();
                    let v = crate::policy::public_support(t, &st.population, &hoods);
                    let band = if v > 0.15 {
                        "talk-support-for"
                    } else if v < -0.15 {
                        "talk-support-against"
                    } else {
                        "talk-support-split"
                    };
                    args.insert("support_key".into(), band.into());
                }
            }
            _ => {}
        }
    }

    /// Do all conditions hold? Collects their arguments.
    pub fn all_hold(&self, cs: &[SituationCondition], args: &mut Args) -> bool {
        cs.iter().all(|c| self.holds(c, args))
    }

    /// Does a condition hold here? Adds the arguments it provides when it does.
    pub fn holds(&self, c: &SituationCondition, args: &mut Args) -> bool {
        use SituationCondition as U;
        let st = self.st;
        let pol = &st.politics;
        match c {
            U::Crisis { kind, .. } => {
                if !crate::policy::situation_holds(st, self.content, c, self.town.0) {
                    return false;
                }
                if let Some(k) = self.content.crises.iter().find(|x| &x.id == kind) {
                    args.insert("crisis_key".into(), k.name_key.clone());
                }
                true
            }
            U::Indicator { .. } | U::PlantClosure { .. } | U::Disaster { .. } => {
                crate::policy::situation_holds(st, self.content, c, self.town.0)
            }
            U::Occupation { slot, occupations } => self
                .person(slot)
                .and_then(|p| characters::knows::role_key(st, p))
                .is_some_and(|r| occupations.contains(&r)),
            U::Trait { slot, traits } => self
                .person(slot)
                .and_then(|p| pol.characters.get(p))
                .is_some_and(|ch| ch.traits.iter().any(|t| traits.contains(t))),
            U::Personality {
                slot,
                axis,
                min,
                max,
            } => self
                .person(slot)
                .and_then(|p| characters::profile(pol, &st.world, p))
                .is_some_and(|p| {
                    let x = &p.personality;
                    let v = match axis.as_str() {
                        "ambition" => x.ambition,
                        "integrity" => x.integrity,
                        "warmth" => x.warmth,
                        "boldness" => x.boldness,
                        "loyalty" => x.loyalty,
                        _ => return false,
                    };
                    in_range(v as f64, *min, *max)
                }),
            U::Goal { slot, goal } => self.person(slot).is_some_and(|p| {
                pol.characters
                    .goals
                    .iter()
                    .any(|g| g.holder == p && &g.goal == goal)
            }),
            U::Opinion { slot, min, max } => self.person(slot).is_some_and(|p| {
                p != self.me && in_range(characters::opinion(pol, p, self.me) as f64, *min, *max)
            }),
            U::Trust { slot, min, max } => self.person(slot).is_some_and(|p| {
                p != self.me
                    && in_range(
                        characters::trust(pol, self.content, p, self.me) as f64,
                        *min,
                        *max,
                    )
            }),
            U::Remembers { slot, kinds } => {
                let Some(p) = self.person(slot) else {
                    return false;
                };
                let date = st.calendar.date;
                let Some(m) = characters::salient_memories(pol, p, Some(self.me), date, usize::MAX)
                    .into_iter()
                    .find(|m| kinds.iter().any(|k| k == m.kind.key()))
                else {
                    return false;
                };
                let subject = match m.subject {
                    Some(characters::Subject::Policy { id }) => st
                        .policy
                        .instances
                        .get(id as usize)
                        .and_then(|i| self.content.policies.get(&i.template))
                        .map(|t| t.name_key.clone()),
                    _ => None,
                }
                .unwrap_or_else(|| SUBJECT_FALLBACK.into());
                args.insert("memory_key".into(), m.kind.fluent_key());
                args.insert("subject_key".into(), subject);
                args.insert("year".into(), m.date.calendar().year.to_string());
                true
            }
            U::Favour {
                slot,
                owed_to_player,
            } => {
                let Some(p) = self.person(slot).filter(|p| *p != self.me) else {
                    return false;
                };
                let n = if *owed_to_player {
                    characters::favours_owed(pol, p, self.me)
                } else {
                    characters::favours_owed(pol, self.me, p)
                };
                if n == 0 {
                    return false;
                }
                args.insert("n".into(), n.to_string());
                true
            }
            U::IsBroker { slot, is } => {
                let Some(p) = self.person(slot) else {
                    return false;
                };
                let yes = self.broker() == Some(p);
                if yes && *is {
                    self.broker_args(p, args);
                }
                yes == *is
            }
            U::IsRival { slot, is } => {
                let Some(p) = self.person(slot) else {
                    return false;
                };
                let yes = self.rival() == Some(p);
                if yes && *is {
                    args.insert("rival".into(), self.name_of(p));
                }
                yes == *is
            }
            U::Tells { subject, min } => {
                let Some(s) = self.subject(subject) else {
                    return false;
                };
                if self.told(&s) < *min {
                    return false;
                }
                self.subject_args(&s, args);
                true
            }
            U::PlayerKnows { subject, min, max } => {
                // Something that does not exist here (a town without a
                // broker) is unknown to the player, and gives no arguments.
                let s = self.subject(subject);
                let l = s
                    .as_ref()
                    .map(|s| knowledge::knows(st, s))
                    .unwrap_or_default();
                if min.is_some_and(|a| l < a) || max.is_some_and(|z| l > z) {
                    return false;
                }
                if let Some(s) = &s {
                    self.subject_args(s, args);
                }
                true
            }
            U::Any { of } => {
                for x in of {
                    let mut a = Args::new();
                    if self.holds(x, &mut a) {
                        args.extend(a);
                        return true;
                    }
                }
                false
            }
        }
    }
}

/// The slot-free reading of a situation condition in town `m` (a policy's
/// unlock, a family trigger; `policy::situation_holds` delegates its new
/// variants here). Conversation conditions never hold in a town's
/// situation: slot-relative ones need a speaker, and the player's ledger
/// (`player_knows`, `tells`) never decides whether something happens
/// (DEC-091: the world does not wait for the player; validation refuses
/// them there too).
pub fn holds_in_town(
    state: &State,
    content: &ContentBundle,
    c: &SituationCondition,
    m: u32,
) -> bool {
    match c {
        SituationCondition::Any { of } => of
            .iter()
            .any(|x| crate::policy::situation_holds(state, content, x, m)),
        _ => false,
    }
}
