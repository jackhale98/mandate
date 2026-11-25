//! A conversation's context (§3.3): the slots (`speaker`, `about`,
//! `player`; the place is `here`) and what the speaker will tell. Its
//! conditions are the one condition language, decided by the one evaluator
//! (`crate::condition`) with this conversation bound, with the Fluent
//! arguments each condition provides. Read-only.

use super::AboutRef;
use crate::characters::{self, CharacterId};
use crate::places::Location;
use crate::player::knowledge::{Level, PolicyRef, Subject};
use crate::state::State;
use crate::world::{MunicipalityId, NeighborhoodId};
use mandate_content::conversation::{HoodOf, SubjectRef};
use mandate_content::{Condition, ContentBundle};

pub type Args = crate::condition::Args;

/// How a speaker is tied to a neighbourhood (the `about` condition's
/// `home`, `office`, `works`, `here`): whether they have reason to know it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HoodTies {
    pub home: bool,
    /// Inside the jurisdiction (or district) of a post they hold.
    pub office: bool,
    pub works: bool,
    /// The conversation happens there.
    pub here: bool,
}

impl HoodTies {
    pub fn any(self) -> bool {
        self.home || self.office || self.works || self.here
    }
}

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
        crate::condition::concern_args(self.content, i, args);
    }

    /// A scope's worries as the notebook records them (felt:
    /// `concerns::ranked`), strongest first, so what a speaker says of a
    /// place is what the player is told they learned.
    fn felt(&self, scope: crate::concerns::Scope<'_>) -> Vec<(usize, f64)> {
        let st = self.st;
        let ranking = crate::concerns::ranked(
            &st.population,
            self.content,
            &st.world,
            scope,
            mandate_content::condition::Worry::Felt,
        );
        crate::concerns::top(&ranking, usize::MAX).collect()
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
        characters::knows::workplace(self.st, c, self.town)
    }

    /// How the speaker is tied to a neighbourhood: they live there, hold a
    /// post over it, work there, or are talking there now.
    pub fn ties(&self, hood: NeighborhoodId) -> HoodTies {
        let st = self.st;
        HoodTies {
            home: self.hood(HoodOf::SpeakerHome) == Some(hood),
            office: characters::knows::holds_post_over(st, self.speaker, hood),
            works: characters::knows::works_in(st, self.speaker, hood),
            here: crate::places::hood_of(st, self.here) == Some(hood),
        }
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
    pub(crate) fn broker_args(&self, b: CharacterId, args: &mut Args) {
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

    /// The speaker's role facts (M6 note 26), so a line can be said from
    /// where they stand: `$post` and `$body` (their post, and the body that
    /// votes at its tier, in the constitution's words; only for someone in a
    /// post), `$work` (where they work; the town for someone with no
    /// workplace) and `$mystake` (the industry their occupation speaks for:
    /// the town's own where it is one of theirs; only for an occupation with
    /// a stake). Which of these a line may rely on is decided by its
    /// conditions (`conversation::role_fact_args`).
    pub fn role_args(&self, args: &mut Args) {
        use mandate_content::conversation::{body_talk_key, post_talk_key};
        let st = self.st;
        // The visible power where the conversation is (every line's
        // `$office` and `$chamber`): the tier's executive and legislature.
        if let Some(c) = self.content.constitutions.get(&st.meta.constitution) {
            let tier = crate::world::Jurisdiction::town(self.town).tier();
            if let Some(o) = crate::politics::executive(c, tier) {
                args.insert("office_key".into(), post_talk_key(&o.id));
            }
            if let Some(b) = crate::politics::legislature(c, tier) {
                args.insert("chamber_key".into(), body_talk_key(&b.id));
            }
        }
        let ch = st.politics.characters.get(self.speaker);
        let post = ch
            .and_then(|c| c.politician)
            .and_then(|p| st.politics.get(p))
            .and_then(|p| p.role.post());
        if let Some(post) = post
            && let Some(c) = self.content.constitutions.get(&st.meta.constitution)
        {
            if let Some(w) = post.word(c) {
                args.insert("post_key".into(), post_talk_key(w));
            }
            if let Some(b) = crate::politics::legislature(c, post.at().tier()) {
                args.insert("body_key".into(), body_talk_key(&b.id));
            }
        }
        match self.workplace(self.speaker) {
            Some(p) => self.place_arg(p, "work", args),
            None => {
                if let Some(m) = st.world.municipalities.get(self.town.index()) {
                    args.insert("work".into(), m.name.clone());
                }
            }
        }
        let drivers = ch
            .and_then(|c| c.named.as_deref())
            .and_then(|n| self.content.occupation(&n.occupation))
            .map(|o| o.stake.drivers.as_slice())
            .unwrap_or(&[]);
        if !drivers.is_empty() {
            let town = self
                .town_character()
                .and_then(|t| t.industry.as_deref())
                .and_then(|s| self.content.sector(s))
                .filter(|s| drivers.contains(&s.driver));
            let mine = town.or_else(|| {
                self.content
                    .sectors
                    .iter()
                    .find(|s| drivers.contains(&s.driver))
            });
            let key = mine
                .and_then(|s| s.talk_key.clone())
                .unwrap_or_else(|| STAKE_FALLBACK.into());
            args.insert("mystake_key".into(), key);
        }
    }

    /// The arguments a subject provides (`SubjectRef::args`).
    pub(crate) fn subject_args(&self, s: &Subject, args: &mut Args) {
        let st = self.st;
        match s {
            Subject::TownConcerns { town } => {
                let top = self.felt(crate::concerns::Scope::town(*town));
                if let Some((i, _)) = top.first() {
                    self.concern_args(*i, args);
                }
            }
            Subject::HoodConcerns { hood } => {
                if st.world.neighborhoods.get(hood.index()).is_some() {
                    hood_arg(st, *hood, "hood", args);
                    let felt = self.felt(crate::concerns::Scope::Neighbourhood(*hood));
                    if let Some((i, _)) = felt.first() {
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

    /// Do all conditions hold? Collects their arguments (the one
    /// evaluator, with this conversation bound).
    pub fn all_hold(&self, cs: &[Condition], args: &mut Args) -> bool {
        crate::condition::on_state(self.st, self.content, |v| {
            let cx = crate::condition::Ctx::talk(v, self);
            crate::condition::all_hold(&cx, cs, args)
        })
    }

    /// Does a condition hold here? Adds the arguments it provides when it does.
    pub fn holds(&self, c: &Condition, args: &mut Args) -> bool {
        crate::condition::on_state(self.st, self.content, |v| {
            let cx = crate::condition::Ctx::talk(v, self);
            crate::condition::holds(&cx, c, args)
        })
    }
}

/// The Fluent key naming a neighbourhood that has its town's name: the
/// town centre ("Larkowick town centre"; `$centre_town`). The stored name
/// stays; only how it is named to the player changes (M6 note 19).
pub const HOOD_CENTRE_KEY: &str = "ui-hood-centre";

/// The town whose name a neighbourhood shares (its centre), if it does.
pub fn centre_town(st: &State, hood: NeighborhoodId) -> Option<&str> {
    let h = st.world.neighborhoods.get(hood.index())?;
    let m = st.world.municipalities.get(h.municipality.index())?;
    (m.name == h.name).then_some(m.name.as_str())
}

/// A neighbourhood's name as an argument: `name` (the stored name) and,
/// for the centre, `<name>_key` (`ui-hood-centre`, which the client reads in
/// place of the name) with `$centre_town`.
pub fn hood_arg(st: &State, hood: NeighborhoodId, name: &str, args: &mut Args) {
    let Some(h) = st.world.neighborhoods.get(hood.index()) else {
        return;
    };
    args.insert(name.into(), h.name.clone());
    match centre_town(st, hood) {
        Some(town) => {
            args.insert(format!("{name}_key"), HOOD_CENTRE_KEY.into());
            args.insert("centre_town".into(), town.into());
        }
        None => {
            args.remove(&format!("{name}_key"));
        }
    }
}

/// The Fluent key naming a place of this kind in talk ("the union hall"),
/// for places without a proper name.
pub fn place_talk_key(kind: crate::world::PlaceKind) -> String {
    format!("talk-place-{}", kind.key())
}
