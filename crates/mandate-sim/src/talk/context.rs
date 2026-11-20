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
use mandate_content::conversation::{ConcernOf, HoodOf, SubjectRef};
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

    /// `$concern_key` and `$concern_talk_key` (the noun phrase of talk, or
    /// the name without one) for the concern at registry index `i`.
    fn concern_args(&self, i: usize, args: &mut Args) {
        if let Some(c) = self.content.concerns.get(i) {
            args.insert("concern_key".into(), c.name_key.clone());
            let talk = c.talk_key.as_ref().unwrap_or(&c.name_key);
            args.insert("concern_talk_key".into(), talk.clone());
        }
    }

    /// A town's worries as the notebook records them (felt across all its
    /// neighbourhoods: `concerns::felt_concerns`), strongest first, so what
    /// a speaker says of the town is what the player is told they learned.
    fn town_felt(&self, town: MunicipalityId) -> Vec<(usize, f64)> {
        let st = self.st;
        let hoods: Vec<usize> = st.world.hoods_of(town).collect();
        let mut v =
            crate::concerns::felt_concerns(&st.population, self.content, town.index(), &hoods);
        v.retain(|(_, w)| *w > 0.0);
        v
    }

    /// The registry index of a top concern (one that worries at all).
    fn top_concern(&self, of: ConcernOf) -> Option<usize> {
        let st = self.st;
        let top = match of {
            ConcernOf::Town => self.town_felt(self.town),
            ConcernOf::Speaker => {
                let home = self.hood(HoodOf::SpeakerHome)?;
                let m = st.world.neighborhoods.get(home.index())?.municipality;
                crate::concerns::felt_concerns(
                    &st.population,
                    self.content,
                    m.index(),
                    &[home.index()],
                )
            }
        };
        top.first().filter(|(_, l)| *l > 0.0).map(|(i, _)| *i)
    }

    /// A place's name as an argument: its proper name in `name`, or, for a
    /// place without one, its kind's noun phrase of talk (`talk-place-<kind>`,
    /// "the union hall") as the Fluent key `<name>_key`.
    pub fn place_arg(&self, p: &crate::world::Place, name: &str, args: &mut Args) {
        if p.name.is_empty() {
            args.remove(name);
            args.insert(format!("{name}_key"), place_talk_key(p.kind));
        } else {
            args.remove(&format!("{name}_key"));
            args.insert(name.into(), p.name.clone());
        }
    }

    /// Where a person works: their own place (a named character's), else
    /// where their post sits (DEC-098), in this town. `None` for someone
    /// with neither.
    pub fn workplace(&self, c: CharacterId) -> Option<&crate::world::Place> {
        let st = self.st;
        let ch = st.politics.characters.get(c)?;
        if let Some(p) = ch
            .named
            .as_deref()
            .and_then(|n| n.place)
            .and_then(|p| st.world.places.get(p.index()))
        {
            return Some(p);
        }
        let kind = ch
            .politician
            .and_then(|p| st.politics.get(p))
            .and_then(|p| p.role.post())
            .and_then(|post| crate::politics::workplace_of(&st.politics, post))?;
        st.world
            .places
            .iter()
            .find(|p| p.municipality == self.town && p.kind == kind)
    }

    /// Where a person is found, as the argument `name`: their workplace,
    /// else their office (the town hall for the mayor, the council
    /// chamber), else the town.
    fn place_of(&self, c: CharacterId, name: &str, args: &mut Args) {
        let st = self.st;
        let named_place = st
            .politics
            .characters
            .get(c)
            .and_then(|x| x.named.as_deref())
            .and_then(|n| n.place)
            .and_then(|p| st.world.places.get(p.index()));
        let office = || {
            // Where their post works (DEC-098), else where the town's
            // legislature sits.
            let at = crate::world::Jurisdiction::town(self.town);
            let post_place = st
                .politics
                .characters
                .get(c)
                .and_then(|x| x.politician)
                .and_then(|p| st.politics.get(p))
                .and_then(|p| p.role.post())
                .and_then(|post| crate::politics::workplace_of(&st.politics, post));
            let kind = post_place.or_else(|| {
                let c = self.content.constitutions.get(&st.meta.constitution)?;
                let body = crate::politics::legislature(c, at.tier())?;
                let key = mandate_content::PostKey::new(&body.id)?;
                crate::politics::workplace_of(
                    &st.politics,
                    crate::politics::Post::Seat {
                        body: key,
                        at,
                        constituency: None,
                    },
                )
            })?;
            st.world
                .places
                .iter()
                .find(|p| p.municipality == self.town && p.kind == kind)
        };
        match named_place.or_else(office) {
            Some(p) => self.place_arg(p, name, args),
            None => {
                let town = st
                    .world
                    .municipalities
                    .get(self.town.index())
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                args.insert(name.into(), town);
            }
        }
    }

    /// The broker's arguments: `$broker`, `$broker_place`, `$stake_key`.
    fn broker_args(&self, b: CharacterId, args: &mut Args) {
        args.insert("broker".into(), self.name_of(b));
        self.place_of(b, "broker_place", args);
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
                let top = self.town_felt(*town);
                if let Some((i, _)) = top.first() {
                    self.concern_args(*i, args);
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
                    if let Some((i, _)) = felt.first().filter(|(_, w)| *w > 0.0) {
                        self.concern_args(*i, args);
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
                .and_then(|p| characters::knows::role_key(st, self.content, p))
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
                // What it was about, and what kind of thing that is
                // (`$subject_kind`: `bill`, a policy's name; `decision`, a
                // decision's title, a noun phrase; `none`), so a line can
                // say it naturally or leave it out.
                let subject = match m.subject {
                    Some(characters::Subject::Policy { id }) => st
                        .policy
                        .instances
                        .get(id as usize)
                        .and_then(|i| self.content.policies.get(&i.template))
                        .map(|t| (t.name_key.clone(), "bill")),
                    Some(characters::Subject::Decision { id }) => st
                        .narrative
                        .decision(crate::narrative::DecisionId(id))
                        .and_then(|d| self.content.dilemma(&d.template))
                        .map(|t| (t.title_key.clone(), "decision")),
                    _ => None,
                };
                let (subject, kind) = subject.unwrap_or_else(|| (SUBJECT_FALLBACK.into(), "none"));
                args.insert("memory_key".into(), m.kind.fluent_key());
                args.insert("subject_key".into(), subject);
                args.insert("subject_kind".into(), kind.into());
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
                let mut a = Args::new();
                self.subject_args(&s, &mut a);
                // Worries that worry nobody are no answer.
                if matches!(
                    s,
                    Subject::TownConcerns { .. } | Subject::HoodConcerns { .. }
                ) && !a.contains_key("concern_key")
                {
                    return false;
                }
                args.extend(a);
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
            U::Voice { voices } => {
                super::reply::voice(self).is_some_and(|v| voices.contains(&v.id))
            }
            U::About {
                kinds,
                places,
                workplace,
                home,
                is,
            } => {
                let holds = self.about.is_some_and(|a| {
                    if !kinds.is_empty() && !kinds.contains(&a.kind()) {
                        return false;
                    }
                    let place = match a {
                        AboutRef::Place { place } => st.world.places.get(place.index()),
                        _ => None,
                    };
                    if !places.is_empty()
                        && !place.is_some_and(|p| places.iter().any(|k| k == p.kind.key()))
                    {
                        return false;
                    }
                    if let Some(h) = home {
                        let mine = matches!(a, AboutRef::Hood { .. })
                            && self.hood(HoodOf::About) == self.hood(HoodOf::SpeakerHome);
                        if mine != *h {
                            return false;
                        }
                    }
                    match workplace {
                        None => true,
                        Some(w) => {
                            let mine = place.is_some_and(|p| {
                                self.workplace(self.speaker).is_some_and(|q| q.id == p.id)
                            });
                            mine == *w
                        }
                    }
                });
                holds == *is
            }
            U::Concern { of, concerns } => {
                let Some(i) = self.top_concern(*of) else {
                    return false;
                };
                if !self
                    .content
                    .concerns
                    .get(i)
                    .is_some_and(|c| concerns.contains(&c.id))
                {
                    return false;
                }
                self.concern_args(i, args);
                true
            }
        }
    }
}

/// The Fluent key naming a place of this kind in talk ("the union hall"),
/// for places without a proper name.
pub fn place_talk_key(kind: crate::world::PlaceKind) -> String {
    format!("talk-place-{}", kind.key())
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
        // The town's top concern (one that worries at all).
        SituationCondition::Concern {
            of: ConcernOf::Town,
            concerns,
        } => crate::concerns::top_concerns(&state.population, m as usize)
            .first()
            .filter(|(_, l)| *l > 0.0)
            .and_then(|(i, _)| content.concerns.get(*i))
            .is_some_and(|x| concerns.contains(&x.id)),
        _ => false,
    }
}
