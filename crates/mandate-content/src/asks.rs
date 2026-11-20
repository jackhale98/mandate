//! Asks and promises (DEC-097 §2; ENGAGEMENT §3): concrete requests with a
//! deadline, made to the player by someone with a reason in the live state,
//! and the public promises a candidate makes to a constituency's voters.
//!
//! Three content kinds: `ask_params` (the sliding rate, caps, casework and
//! promise numbers), `asks` (ask templates) and `promises` (promise
//! templates). The engine is `mandate_sim::asks`.
//!
//! **Roles.** An ask's asker is cast like a dilemma role (any [`Cast`]) under
//! the role id `asker`; the asker's home neighbourhood is the implicit role
//! `asker_hood`. Further roles (a bill, a neighbourhood, an official) are
//! listed in `roles`. Promises have no cast: their asker is the voters of
//! the constituency the player is running in, and a promise's chosen answer
//! (a menu argument) is `answer`.
//!
//! **Tiers (DEC-098).** Offices and bodies are named only through the
//! constitution (a colleague is `{ "type": "colleague" }`, the executive
//! `{ "type": "office" }`); no ask names an office word.
//!
//! Adding an ask: `docs/development/EXTENDING.md`, "Adding an ask".

use crate::bundle::ContentBundle;
use crate::condition::{Condition, ConditionUse};
use crate::narrative::{Cast, EffectDef, RoleDef, Scope};
use crate::types::ContentKey;
use crate::validate::Issue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The implicit role of the asker.
pub const ROLE_ASKER: &str = "asker";
/// The implicit role of the asker's home neighbourhood.
pub const ROLE_ASKER_HOOD: &str = "asker_hood";
/// The implicit role of a promise's chosen answer (a policy template).
pub const ROLE_ANSWER: &str = "answer";
/// The one tag `against_all` knows: a bill that raises the town's taxes
/// (a `local_tax_rate` mechanism set above zero).
pub const TAG_TAX_RISE: &str = "tax_rise";
/// Deadlines are within these bounds (days).
pub const DEADLINE_DAYS: [u32; 2] = [7, 900];

fn one() -> f64 {
    1.0
}

// --- Parameters ----------------------------------------------------------------------

/// The asks system's numbers (`content/base/asks/params.jsonc`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AskParams {
    /// Offered and accepted asks at once (a hard cap; promises are not counted).
    pub max_open: u8,
    /// The base rate of new offers a month, governing and in a campaign: the
    /// two intent curves multiply it.
    pub new_per_month: f64,
    pub new_per_month_campaign: f64,
    /// Open asks → factor (points `[x, factor]`, linear between them).
    pub intent_by_open: Vec<[f64; 2]>,
    /// Days since the last offer → factor.
    pub intent_by_quiet: Vec<[f64; 2]>,
    /// An offer lapses unanswered (as declined) after this many days.
    pub respond_days: u32,
    /// Days before the same person (or party) asks again.
    pub per_asker_cooldown_days: u32,
    /// A person whose earlier ask the player kept is this much more likely
    /// to ask again; one whose ask was broken, this much (callbacks).
    pub asker_kept_mult: f64,
    pub asker_broken_mult: f64,
    /// Chance a canvass (or a conversation) brings a doorstep ask.
    pub doorstep_chance: f64,
    /// Resolved asks kept for views and the lab.
    pub history: u32,
    /// The player's deed log (ENGAGEMENT §3.6).
    pub deeds_kept: u32,
    /// How far back "the player asked around about someone" reaches (days).
    #[serde(default = "talked_about_days")]
    pub talked_about_days: u32,
    pub casework: CaseworkParams,
    pub promises: PromiseParams,
}

fn talked_about_days() -> u32 {
    30
}

/// Casework (ENGAGEMENT §3.4): "Sort it out" at the town hall, one slot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseworkParams {
    pub success_in_office: f64,
    /// Out of office the player can only chase it up.
    pub success_out_of_office: f64,
}

/// Campaign promises (ENGAGEMENT §3.5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromiseParams {
    pub max_per_campaign: u8,
    /// Persuasion a promise adds where its worry is felt: × the felt level
    /// of the worry in each neighbourhood of the race (× `any_salience` for
    /// a promise that answers no worry), approval × room.
    pub persuasion: f64,
    pub any_salience: f64,
    /// × this per public promise the player has broken.
    pub broken_discount: f64,
}

impl Default for AskParams {
    fn default() -> Self {
        AskParams {
            max_open: 4,
            new_per_month: 1.5,
            new_per_month_campaign: 2.5,
            intent_by_open: vec![[0.0, 2.0], [1.0, 1.3], [2.0, 1.0], [3.0, 0.5], [4.0, 0.0]],
            intent_by_quiet: vec![[0.0, 0.0], [7.0, 0.5], [21.0, 1.0], [42.0, 1.5]],
            respond_days: 7,
            per_asker_cooldown_days: 60,
            asker_kept_mult: 1.5,
            asker_broken_mult: 0.5,
            doorstep_chance: 0.35,
            history: 60,
            deeds_kept: 120,
            talked_about_days: 30,
            casework: CaseworkParams {
                success_in_office: 0.85,
                success_out_of_office: 0.5,
            },
            promises: PromiseParams {
                max_per_campaign: 3,
                persuasion: 0.04,
                any_salience: 0.5,
                broken_discount: 0.5,
            },
        }
    }
}

/// A curve of `[x, factor]` points, linear between them, flat outside.
pub fn interpolate(points: &[[f64; 2]], x: f64) -> f64 {
    let Some(first) = points.first() else {
        return 1.0;
    };
    if x <= first[0] {
        return first[1];
    }
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        if x <= b[0] {
            let span = b[0] - a[0];
            if span <= 0.0 {
                return b[1];
            }
            return a[1] + (b[1] - a[1]) * (x - a[0]) / span;
        }
    }
    points.last().map_or(1.0, |p| p[1])
}

// --- Ask templates -------------------------------------------------------------------

/// Where an offer comes from (ENGAGEMENT §3.2 "Comes from").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AskFrom {
    /// The weekly draw at the sliding rate. An asker cast `goal_holder` is
    /// someone whose NPC goal names this ask (`goals.jsonc` `ask`).
    #[default]
    Week,
    /// A canvass: someone on the doorstep of the canvassed neighbourhood
    /// (`doorstep_chance` a canvass), cast among its residents.
    Doorstep,
    /// A conversation: the person the player talked with
    /// (`doorstep_chance` a conversation).
    Talk,
    /// The player asked around about someone (a conversation about them
    /// within `talked_about_days`): that person asks.
    TalkedAbout,
}

/// Who the asker speaks for. A party's ask is voiced by the person cast as
/// the asker (its chair), and remembered as the party's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnBehalf {
    #[default]
    Person,
    Party,
}

/// How the player must vote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NeedVote {
    Yes,
    No,
    /// As the player's party expects on the bill.
    WithParty,
}

/// What a statement must be about.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StatementDef {
    /// The town's live crisis: a general statement while it lasts, or one on
    /// a bill that answers it.
    Crisis,
    /// A worry: a general statement, or one on a bill that answers it.
    Concern { concern: ContentKey },
    /// The bill cast in `role`.
    Bill { role: String },
    /// Any statement.
    Any,
}

/// Which bills a need counts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BillsDef {
    /// Bills of these policy templates.
    Templates { templates: Vec<ContentKey> },
    /// A template that answers the town's live crisis (its `policies`).
    CrisisAnswer,
    /// A template that answers this worry (the `policies` of the crises
    /// that sharpen it).
    ConcernAnswer { concern: ContentKey },
    /// A promise's chosen answer (its `answer` argument).
    Chosen,
}

/// What keeps an ask: an ordinary command the engine checks (DEC-032: no
/// fetch quests), or casework. An ask's needs are met in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum NeedDef {
    /// A declared vote on the bill cast in `role` (a bill before the
    /// council); due by its vote unless `deadline_days` says otherwise.
    Vote { role: String, vote: NeedVote },
    /// No yes vote on (and no sponsoring of) any bill with this tag until
    /// the deadline (`tax_rise`): kept at the deadline unless broken.
    AgainstAll { tag: String },
    /// A press statement.
    Statement { about: StatementDef },
    /// A town hall in the neighbourhood cast in `role`; with
    /// `constituency`, one in every neighbourhood of the constituency.
    TownHall {
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        constituency: bool,
    },
    /// A canvass of the neighbourhood cast in `role` (`or_town_hall`: a town
    /// hall there counts too).
    Canvass {
        role: String,
        #[serde(default)]
        or_town_hall: bool,
    },
    /// Be at a place of one of these kinds in the town (the first such
    /// place; in the neighbourhood cast in `role`, if it has one).
    Visit {
        kinds: Vec<String>,
        #[serde(default)]
        role: Option<String>,
    },
    /// A conversation with the person cast in `role`.
    Talk { role: String },
    /// Introduce a bill of these templates.
    Introduce { bills: BillsDef },
    /// A bill of these templates becomes law (`sponsor_player`: the player's
    /// own). A promise counts a failed vote on the player's own bill as
    /// half kept (void: no cost, no reward).
    Enacted {
        bills: BillsDef,
        #[serde(default)]
        sponsor_player: bool,
    },
    /// "Sort it out" at the town hall (`Action::Casework`).
    Casework,
    /// None of the player's own hidden facts comes to light before the
    /// deadline: kept at the deadline unless broken.
    NoSurfacedHiddenFact,
    /// The player does not leak about the person cast in `role` (or tip
    /// the press, E9) before the deadline: kept at the deadline unless broken.
    Quiet { role: String },
}

impl NeedDef {
    /// Kept by not acting: only the deadline keeps it, and only alone.
    pub fn watch(&self) -> bool {
        matches!(
            self,
            NeedDef::AgainstAll { .. } | NeedDef::NoSurfacedHiddenFact | NeedDef::Quiet { .. }
        )
    }

    /// Roles it names.
    pub fn roles(&self) -> Vec<&str> {
        match self {
            NeedDef::Vote { role, .. }
            | NeedDef::Canvass { role, .. }
            | NeedDef::Talk { role }
            | NeedDef::Quiet { role } => vec![role.as_str()],
            NeedDef::TownHall { role, .. } | NeedDef::Visit { role, .. } => {
                role.iter().map(|s| s.as_str()).collect()
            }
            NeedDef::Statement {
                about: StatementDef::Bill { role },
            } => vec![role.as_str()],
            _ => Vec::new(),
        }
    }
}

/// An ask template (`content/base/asks/asks.jsonc`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AskDef {
    pub id: ContentKey,
    /// The ask in a few words ("Say something about the closure").
    pub name_key: String,
    /// Who asks (the role `asker`): a person cast like a dilemma role.
    pub asker: Cast,
    #[serde(default)]
    pub on_behalf: OnBehalf,
    /// Where offers come from (default: the weekly draw).
    #[serde(default = "week")]
    pub from: Vec<AskFrom>,
    /// Further roles (a bill, a neighbourhood, an official), cast after the asker.
    #[serde(default)]
    pub roles: Vec<RoleDef>,
    /// U1's conditions on the live state and the cast (never `knows`:
    /// asks are aimed at the player, DEC-091).
    #[serde(default)]
    pub triggers: Vec<Condition>,
    pub needs: Vec<NeedDef>,
    /// Days to keep it, drawn in [min, max] (the lower half in a term after
    /// a re-election). Absent: until the vote on the bill a `vote` need names.
    #[serde(default)]
    pub deadline_days: Option<[u32; 2]>,
    #[serde(default = "one")]
    pub weight: f64,
    /// Days before this template is offered again.
    pub cooldown_days: u32,
    /// Not offered when the player did what its first need asks (a deed in
    /// the deed log) within this many days ("no town hall there in 180 days").
    #[serde(default)]
    pub fresh_days: Option<u32>,
    /// Others see it: breaking it is public (approval, Standing, the press).
    #[serde(default)]
    pub public: bool,
    /// A big ask: kept, a favour is owed to the player; accepting it ends
    /// the director's relax phase.
    #[serde(default)]
    pub big: bool,
    /// Days since play began before it is offered (ENGAGEMENT §4.7).
    #[serde(default)]
    pub min_career_days: u32,
    /// Effects when kept, broken and declined (or left unanswered). A subset
    /// of the narrative effects: `opinion`, `approval`, `standing`, `record`,
    /// `favour`, `learn`, `money`.
    pub keep: Vec<EffectDef>,
    #[serde(rename = "break")]
    pub broken: Vec<EffectDef>,
    #[serde(default)]
    pub decline: Vec<EffectDef>,
    /// The situation, with the cast (and `$history`: `kept`, `broken` or
    /// `none`, the asker's last ask) as arguments.
    pub detail_key: String,
    /// What keeps it ("Vote yes on { $bill_key }").
    pub need_key: String,
    pub kept_key: String,
    pub broken_key: String,
}

fn week() -> Vec<AskFrom> {
    vec![AskFrom::Week]
}

impl AskDef {
    pub fn comes_from(&self, f: AskFrom) -> bool {
        self.from.contains(&f)
    }

    /// Every role id: the asker, the asker's neighbourhood, then `roles`.
    pub fn role_ids(&self) -> Vec<&str> {
        [ROLE_ASKER, ROLE_ASKER_HOOD]
            .into_iter()
            .chain(self.roles.iter().map(|r| r.id.as_str()))
            .collect()
    }
}

// --- Promise templates ---------------------------------------------------------------

/// What a promise answers (validation: every promise names it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PromiseAnswers {
    /// The town's live crisis: the menu offers each template that answers it
    /// (the `answer` argument). Offered only while a crisis is live.
    Crisis,
    /// A worry of the constituency's voters.
    Concern { concern: ContentKey },
    /// No one worry (a promise of presence, of honesty).
    Any,
}

/// When a promise falls due.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PromiseDue {
    /// The end of the term of the post the player is running for.
    Term,
    /// The end of the first full budget year after election day.
    BudgetYear,
    /// This many days after election day.
    Days { days: u32 },
}

/// A promise template (`content/base/asks/promises.jsonc`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromiseDef {
    pub id: ContentKey,
    pub name_key: String,
    pub answers: PromiseAnswers,
    pub needs: Vec<NeedDef>,
    pub due: PromiseDue,
    pub keep: Vec<EffectDef>,
    #[serde(rename = "break")]
    pub broken: Vec<EffectDef>,
    pub detail_key: String,
    pub need_key: String,
    pub kept_key: String,
    pub broken_key: String,
}

// --- Validation ----------------------------------------------------------------------

/// Effects an ask may carry: those the asks system turns into effect
/// requests (OVERVIEW §4.3).
fn allowed_effect(e: &EffectDef) -> bool {
    matches!(
        e,
        EffectDef::Opinion { .. }
            | EffectDef::Approval { .. }
            | EffectDef::Standing { .. }
            | EffectDef::Record { .. }
            | EffectDef::Favour { .. }
            | EffectDef::Learn { .. }
            | EffectDef::Money { .. }
    )
}

/// The Fluent variables (`$name`) one message uses, attributes and
/// variants included.
pub fn message_vars(ftl: &str, key: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut inside = false;
    for line in ftl.lines() {
        let starts_message = line
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic());
        if starts_message {
            inside = line
                .split_once('=')
                .is_some_and(|(id, _)| id.trim() == key);
        } else if line.starts_with('#') {
            inside = false;
        }
        if !inside {
            continue;
        }
        let mut rest = line;
        while let Some(i) = rest.find('$') {
            rest = &rest[i + 1..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !name.is_empty() {
                out.insert(name);
            }
        }
    }
    out
}

/// Arguments every ask and promise message may use besides its roles (as
/// the player reads them: `$crisis` is the text of `crisis_key`).
pub const COMMON_ARGS: [&str; 10] = [
    "player",
    "town",
    "crisis",
    "concern",
    "days",
    "history",
    "place",
    "constituency",
    "count",
    "answer",
];

pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let p = &b.ask_params;
    let err = |issues: &mut Vec<Issue>, owner: &str, msg: &str| issues.push(Issue::error(owner, msg));
    let owner = "ask_params";
    for (name, curve) in [
        ("intent_by_open", &p.intent_by_open),
        ("intent_by_quiet", &p.intent_by_quiet),
    ] {
        if curve.is_empty() {
            err(issues, owner, &format!("{name} needs at least one point"));
        }
        if curve.windows(2).any(|w| w[1][0] <= w[0][0]) {
            err(issues, owner, &format!("{name}: x must increase"));
        }
        if curve.iter().any(|q| !(q[1] >= 0.0) || !q[0].is_finite()) {
            err(issues, owner, &format!("{name}: factors must be ≥ 0"));
        }
    }
    if p.max_open == 0 || p.max_open > 8 {
        err(issues, owner, "max_open must be 1–8");
    }
    if interpolate(&p.intent_by_open, p.max_open as f64) != 0.0 {
        err(issues, owner, "intent_by_open must be 0 at max_open");
    }
    let unit = |v: f64| (0.0..=1.0).contains(&v);
    if !unit(p.doorstep_chance)
        || !unit(p.casework.success_in_office)
        || !unit(p.casework.success_out_of_office)
        || !unit(p.promises.broken_discount)
        || !unit(p.promises.any_salience)
    {
        err(issues, owner, "chances and discounts must be in [0, 1]");
    }
    if !(0.0..=10.0).contains(&p.new_per_month) || !(0.0..=10.0).contains(&p.new_per_month_campaign)
    {
        err(issues, owner, "new_per_month must be in [0, 10]");
    }
    if !(0.0..=0.2).contains(&p.promises.persuasion) {
        err(issues, owner, "promises.persuasion must be in [0, 0.2]");
    }
    if p.asker_kept_mult < 0.0 || p.asker_broken_mult < 0.0 {
        err(issues, owner, "asker multipliers must be ≥ 0");
    }
    if p.history == 0 || p.history > 500 || p.deeds_kept == 0 || p.deeds_kept > 1000 {
        err(issues, owner, "history and deeds_kept must be 1–500 and 1–1000");
    }
    if p.respond_days == 0 || p.respond_days > 60 {
        err(issues, owner, "respond_days must be 1–60");
    }

    let en_text = b.locale_text("en");
    let en = crate::validate::fluent_keys(&en_text);
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for a in &b.asks {
        let owner = a.id.as_str();
        if !ids.insert(owner) {
            err(issues, owner, "ask ids must be unique");
        }
        if !a.id.starts_with("ask.") {
            err(issues, owner, "ask ids start with `ask.`");
        }
        // The asker is a person: a role cast like a dilemma's.
        let asker = RoleDef {
            id: ROLE_ASKER.into(),
            cast: a.asker.clone(),
        };
        if matches!(
            a.asker,
            Cast::Bill { .. } | Cast::Law { .. } | Cast::WardNeighborhood
        ) {
            err(issues, owner, "the asker is a person");
        }
        let raised_by_goal = b.goals.iter().any(|g| g.ask.as_deref() == Some(owner));
        if matches!(a.asker, Cast::GoalHolder) && !raised_by_goal {
            err(
                issues,
                owner,
                "an asker cast `goal_holder` needs a goal that names this ask",
            );
        }
        if a.from.is_empty() {
            err(issues, owner, "`from` needs at least one source");
        }
        let roles: Vec<RoleDef> = std::iter::once(asker).chain(a.roles.clone()).collect();
        let mut seen = crate::narrative::check_roles(owner, b, &roles, true, issues);
        if a.roles.iter().any(|r| r.id == ROLE_ASKER_HOOD) {
            err(issues, owner, "`asker_hood` is implicit");
        }
        seen.push(ROLE_ASKER_HOOD);
        let site = crate::condition::Site::new(owner, ConditionUse::Ask, &seen);
        crate::condition::check_all(b, &site, &a.triggers, issues);
        check_needs(b, owner, &a.needs, &a.roles, &seen, false, issues);
        match a.deadline_days {
            Some([lo, hi]) => {
                if lo > hi || lo < DEADLINE_DAYS[0] || hi > DEADLINE_DAYS[1] {
                    err(
                        issues,
                        owner,
                        &format!(
                            "deadline_days must be within [{}, {}], min ≤ max",
                            DEADLINE_DAYS[0], DEADLINE_DAYS[1]
                        ),
                    );
                }
            }
            None => {
                if !a.needs.iter().any(|n| matches!(n, NeedDef::Vote { .. })) {
                    err(
                        issues,
                        owner,
                        "without deadline_days an ask is due by a vote: it needs a `vote` need",
                    );
                }
            }
        }
        if !(0.0..=10.0).contains(&a.weight) {
            err(issues, owner, "weight must be in [0, 10]");
        }
        if a.cooldown_days > 3650 || a.min_career_days > 3650 {
            err(issues, owner, "cooldown_days and min_career_days must be at most 3650");
        }
        check_effects(owner, &a.keep, &a.broken, &a.decline, &seen, issues);
        // Keys and their arguments (DEC-054).
        let mut provided: BTreeSet<String> = COMMON_ARGS.iter().map(|s| s.to_string()).collect();
        for r in &seen {
            provided.insert(r.to_string());
            provided.insert(format!("{r}_role"));
            provided.insert(format!("{r}_occupation"));
        }
        for key in [&a.name_key, &a.detail_key, &a.need_key, &a.kept_key, &a.broken_key] {
            check_key(owner, key, &en, &en_text, &provided, issues);
        }
    }

    for x in &b.promises {
        let owner = x.id.as_str();
        if !ids.insert(owner) {
            err(issues, owner, "promise ids must be unique");
        }
        if !x.id.starts_with("promise.") {
            err(issues, owner, "promise ids start with `promise.`");
        }
        match &x.answers {
            PromiseAnswers::Concern { concern } if !b.concerns.iter().any(|c| &c.id == concern) => {
                err(issues, owner, &format!("unknown concern `{concern}`"));
            }
            _ => {}
        }
        let chosen = x.needs.iter().any(|n| {
            matches!(
                n,
                NeedDef::Introduce {
                    bills: BillsDef::Chosen
                } | NeedDef::Enacted {
                    bills: BillsDef::Chosen,
                    ..
                }
            )
        });
        if chosen != matches!(x.answers, PromiseAnswers::Crisis) {
            err(
                issues,
                owner,
                "a promise that answers the crisis keeps its chosen answer (`chosen` bills), and only it",
            );
        }
        let seen = vec![ROLE_ANSWER];
        check_needs(b, owner, &x.needs, &[], &seen, true, issues);
        if let PromiseDue::Days { days } = x.due
            && !(DEADLINE_DAYS[0]..=DEADLINE_DAYS[1]).contains(&days)
        {
            err(issues, owner, "a promise's days must be within [7, 900]");
        }
        check_effects(owner, &x.keep, &x.broken, &[], &[], issues);
        let provided: BTreeSet<String> = COMMON_ARGS.iter().map(|s| s.to_string()).collect();
        for key in [&x.name_key, &x.detail_key, &x.need_key, &x.kept_key, &x.broken_key] {
            check_key(owner, key, &en, &en_text, &provided, issues);
        }
    }
}

fn check_key(
    owner: &str,
    key: &str,
    en: &BTreeSet<String>,
    text: &str,
    provided: &BTreeSet<String>,
    issues: &mut Vec<Issue>,
) {
    if !en.contains(key) {
        issues.push(Issue::error(
            owner,
            &format!("missing Fluent key `{key}` in locale en"),
        ));
        return;
    }
    for v in message_vars(text, key) {
        if !provided.contains(&v) {
            issues.push(Issue::error(
                owner,
                &format!("`{key}` uses `${v}`, which this ask does not provide"),
            ));
        }
    }
}

/// Needs name roles of the right kind, real things, and watchers stand alone.
#[allow(clippy::too_many_arguments)]
fn check_needs(
    b: &ContentBundle,
    owner: &str,
    needs: &[NeedDef],
    roles: &[RoleDef],
    seen: &[&str],
    promise: bool,
    issues: &mut Vec<Issue>,
) {
    let err = |issues: &mut Vec<Issue>, msg: &str| issues.push(Issue::error(owner, msg));
    if needs.is_empty() {
        err(issues, "an ask needs at least one need");
    }
    if needs.iter().any(NeedDef::watch) && needs.len() > 1 {
        err(
            issues,
            "a need kept by not acting (`against_all`, `no_surfaced_hidden_fact`, `quiet`) stands alone",
        );
    }
    let cast_of = |r: &str| roles.iter().find(|x| x.id == r).map(|x| &x.cast);
    let is_bill = |r: &str| matches!(cast_of(r), Some(Cast::Bill { .. }));
    let is_hood_only = |r: &str| {
        r == ROLE_ASKER_HOOD || matches!(cast_of(r), Some(Cast::WardNeighborhood))
    };
    let is_person = |r: &str| {
        r == ROLE_ASKER
            || cast_of(r).is_some_and(|c| {
                !matches!(
                    c,
                    Cast::Bill { .. } | Cast::Law { .. } | Cast::WardNeighborhood
                )
            })
    };
    // A need done somewhere names a neighbourhood, or a person (where they live).
    let is_hood = |r: &str| is_hood_only(r) || is_person(r);
    for n in needs {
        for r in n.roles() {
            if !seen.contains(&r) {
                err(issues, &format!("a need names unknown role `{r}`"));
            }
        }
        let bills = |bills: &BillsDef, issues: &mut Vec<Issue>| match bills {
            BillsDef::Templates { templates } => {
                if templates.is_empty() {
                    err(issues, "`templates` needs at least one policy");
                }
                for t in templates {
                    if !b.policies.contains_key(t) {
                        err(issues, &format!("unknown policy `{t}`"));
                    }
                }
            }
            BillsDef::ConcernAnswer { concern } => {
                if !b.concerns.iter().any(|c| &c.id == concern) {
                    err(issues, &format!("unknown concern `{concern}`"));
                }
            }
            BillsDef::Chosen if !promise => {
                err(issues, "only promises keep a chosen answer");
            }
            _ => {}
        };
        match n {
            NeedDef::Vote { role, .. } if !is_bill(role) => {
                err(issues, &format!("a vote need names a bill role (`{role}` is not)"));
            }
            NeedDef::Statement {
                about: StatementDef::Bill { role },
            } if !is_bill(role) => {
                err(issues, &format!("a statement on a bill names a bill role (`{role}` is not)"));
            }
            NeedDef::Statement {
                about: StatementDef::Concern { concern },
            } if !b.concerns.iter().any(|c| &c.id == concern) => {
                err(issues, &format!("unknown concern `{concern}`"));
            }
            NeedDef::TownHall { role, constituency } => match (role, constituency) {
                (Some(r), false) if !is_hood(r) => {
                    err(issues, &format!("a town hall names a neighbourhood role (`{r}` is not)"));
                }
                (None, false) | (Some(_), true) => {
                    err(issues, "a town hall names a neighbourhood role, or `constituency`");
                }
                _ => {}
            },
            NeedDef::Canvass { role, .. } if !is_hood(role) => {
                err(issues, &format!("a canvass names a neighbourhood role (`{role}` is not)"));
            }
            NeedDef::Visit { kinds, role } => {
                if kinds.is_empty() {
                    err(issues, "a visit names place kinds");
                }
                for k in kinds {
                    if !b.places.iter().any(|p| &p.key == k) {
                        err(issues, &format!("unknown place kind `{k}`"));
                    }
                }
                if let Some(r) = role
                    && !is_hood(r)
                {
                    err(issues, &format!("a visit's role is a neighbourhood (`{r}` is not)"));
                }
            }
            NeedDef::Talk { role } | NeedDef::Quiet { role } if !is_person(role) => {
                err(issues, &format!("`{role}` is not a person"));
            }
            NeedDef::AgainstAll { tag } if tag != TAG_TAX_RISE => {
                err(issues, &format!("unknown tag `{tag}` (tags: {TAG_TAX_RISE})"));
            }
            NeedDef::Introduce { bills: x } | NeedDef::Enacted { bills: x, .. } => {
                bills(x, issues);
            }
            _ => {}
        }
    }
}

/// Every ask costs something when broken and gives something when kept
/// (DEC-073), with effects the asks system can request, on roles it has.
fn check_effects(
    owner: &str,
    keep: &[EffectDef],
    broken: &[EffectDef],
    decline: &[EffectDef],
    roles: &[&str],
    issues: &mut Vec<Issue>,
) {
    let err = |issues: &mut Vec<Issue>, msg: &str| issues.push(Issue::error(owner, msg));
    if !keep.iter().any(|e| e.valence().1) {
        err(issues, "keeping an ask needs a gain (DEC-073)");
    }
    if !broken.iter().any(|e| e.valence().0) {
        err(issues, "breaking an ask needs a cost (DEC-073)");
    }
    for e in keep.iter().chain(broken).chain(decline) {
        if !allowed_effect(e) {
            err(
                issues,
                "ask effects are opinion, approval, standing, record, favour, learn or money",
            );
        }
        for r in e.roles() {
            if !roles.contains(&r) {
                err(issues, &format!("an effect names unknown role `{r}`"));
            }
        }
        match e {
            EffectDef::Opinion { delta, memory, .. } => {
                if !(-30.0..=30.0).contains(delta) {
                    err(issues, "an ask's opinion change is within [-30, 30]");
                }
                if let Some(m) = memory
                    && !crate::types::MEMORY_KINDS.contains(&m.as_str())
                {
                    err(issues, &format!("unknown memory kind `{m}`"));
                }
            }
            EffectDef::Approval {
                delta,
                classes,
                scope,
                neighborhood,
            } => {
                if !(-0.05..=0.05).contains(delta) {
                    err(issues, "an ask's approval change is within [-0.05, 0.05]");
                }
                if classes.iter().any(|c| *c > 2) {
                    err(issues, "approval classes are 0, 1, 2");
                }
                if neighborhood.is_some() && *scope == Scope::Town {
                    err(issues, "approval in a neighbourhood has no town scope");
                }
            }
            EffectDef::Standing { delta } | EffectDef::Record { delta } => {
                if !(-5.0..=5.0).contains(delta) {
                    err(issues, "an ask's standing or record change is within [-5, 5]");
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_interpolate_linearly_and_flat_outside() {
        let p = AskParams::default();
        assert_eq!(interpolate(&p.intent_by_open, 0.0), 2.0);
        assert!((interpolate(&p.intent_by_open, 0.5) - 1.65).abs() < 1e-12);
        assert_eq!(interpolate(&p.intent_by_open, 4.0), 0.0);
        assert_eq!(interpolate(&p.intent_by_open, 9.0), 0.0);
        assert_eq!(interpolate(&p.intent_by_quiet, 0.0), 0.0);
        assert!((interpolate(&p.intent_by_quiet, 14.0) - 0.75).abs() < 1e-12);
        assert_eq!(interpolate(&p.intent_by_quiet, 100.0), 1.5);
    }

    #[test]
    fn message_vars_reads_one_message_with_its_variants() {
        let ftl = "a = Hi { $x }\nb = { $history ->\n    [kept] Thanks, { $asker }\n   *[none] Hello\n}\n    .title = { $bill_key }\nc = { $y }\n";
        let v = message_vars(ftl, "b");
        assert_eq!(
            v,
            ["asker", "bill_key", "history"]
                .into_iter()
                .map(String::from)
                .collect()
        );
    }
}
