//! Office views (M3): the player's desk, bills with whip counts, the council,
//! the policy catalogue, and town opinion. Estimates shown here are what the
//! player can know (whip counts are probabilities, REQ-013).

use crate::knowledge::{Gate, KnowledgeLevelDto, gate_concerns, gate_support, gate_ward_support};
use crate::views::DateView;
use mandate_content::{ParamDef, TierKey};
use mandate_sim::actions::{lobby_odds, player_powers};
use mandate_sim::player::knowledge::{Level, Subject};
use mandate_sim::policy::{
    Clause, PolicyInstance, PolicyStatus, executive_lean, legislature_leans, public_support,
    sign_probability, support_by_class, yes_probability,
};
use mandate_sim::politics::Politician;
use mandate_sim::world::{ConstituencyId, MunicipalityId};
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A simulated downside of a policy (DEC-073), for display.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SideEffectView {
    /// `disruption` | `grievance` | `labour_supply`
    pub kind: String,
    /// Who it touches: `low` | `middle` | `high` | `all`.
    pub group: String,
    /// Only older residents (45+).
    pub older: bool,
    /// Size at these parameters (positive = harm).
    pub amount: f64,
    /// The size in words: `small` | `moderate` | `large` | `relief`.
    pub size: String,
}

/// The size of a side effect in words (DEC-073: trade-offs before the choice).
pub fn side_effect_size(amount: f64) -> &'static str {
    let v = amount.abs();
    if amount < 0.0 {
        "relief"
    } else if v < 0.04 {
        "small"
    } else if v < 0.12 {
        "moderate"
    } else {
        "large"
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActionCostView {
    pub action: String,
    pub slots: u32,
    pub health: f64,
}

pub fn side_effects_view(
    t: &mandate_content::PolicyTemplate,
    p: &PolicyInstance,
) -> Vec<SideEffectView> {
    use mandate_content::{ClassKey, SideEffectDef as S};
    let class = |c: Option<ClassKey>| match c {
        Some(ClassKey::Low) => "low",
        Some(ClassKey::Middle) => "middle",
        Some(ClassKey::High) => "high",
        None => "all",
    };
    t.side_effects
        .iter()
        .map(|fx| {
            let amount = mandate_sim::policy::side_effect_amount(fx, p);
            let (group, older, amount) = match fx {
                S::Disruption { .. } => ("all", false, amount),
                S::Grievance {
                    class: c,
                    min_age_band,
                    ..
                } => (class(*c), *min_age_band >= 2, amount),
                // Fewer people working is the harm.
                S::LabourSupply { class: c, .. } => (class(Some(*c)), false, -amount),
            };
            SideEffectView {
                kind: fx.key().into(),
                group: group.into(),
                older,
                amount,
                size: side_effect_size(amount).into(),
            }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TracksView {
    pub viability: f64,
    pub legal_jeopardy: f64,
    pub threat: f64,
    pub standing: f64,
    pub health: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActionLogView {
    pub date: DateView,
    pub action_key: String,
    pub outcome_key: String,
    pub args: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlayerView {
    pub politician: u32,
    pub name: String,
    pub party: Option<u32>,
    /// Fluent key of the title of the player's first post (`role-<word>`),
    /// or `role-private` out of office: derived from `posts`.
    pub role_key: String,
    /// The constitution posts the player holds (DEC-098; one in M6, none
    /// out of office).
    pub posts: Vec<crate::post::PostDto>,
    pub municipality: String,
    pub municipality_id: u32,
    pub ward_number: u32,
    pub ward_id: u32,
    pub slots_total: u32,
    pub slots_left: u32,
    pub late_night_available: bool,
    pub mandate: f64,
    pub money: f64,
    pub staff: u32,
    pub tracks: TracksView,
    pub can_propose: bool,
    pub recent_actions: Vec<ActionLogView>,
    /// Slots this turn after exhaustion (health below 35 / 15 costs a slot).
    pub slot_cap: u32,
    pub press_gaffe_risk: f64,
    pub ads_left: u32,
    pub ad_cost: f64,
    pub action_costs: Vec<ActionCostView>,
    /// Visible work in office (0..100), DEC-076.
    pub record: f64,
    /// Below this Standing the party will not renominate the player.
    pub renomination_standing: f64,
    /// Shady actions with their payoff and risk line (DEC-034, DEC-073).
    pub shady: Vec<crate::media::ShadyOfferView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParamView {
    pub id: String,
    pub name_key: String,
    pub unit_key: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SupportView {
    /// −1..1 by class and overall (turnout-weighted): to 0.1 when the
    /// player knows who supports it, exact in detail, `None` below that
    /// (DEC-091).
    pub low: Option<f64>,
    pub middle: Option<f64>,
    pub high: Option<f64>,
    pub overall: Option<f64>,
    /// The overall figure in a word, always shown: `popular` | `divisive` |
    /// `unpopular` (Fluent `support-band-<band>`).
    pub band: String,
    /// What the player knows of this policy's support.
    pub level: crate::knowledge::KnowledgeLevelDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TemplateView {
    pub id: String,
    pub name_key: String,
    pub desc_key: String,
    /// Budget category in snake case.
    pub category: String,
    pub params: Vec<ParamView>,
    pub implementation_months: u32,
    pub secondary_keys: Vec<String>,
    pub support: SupportView,
    pub est_cost_default: f64,
    pub side_effects: Vec<SideEffectView>,
    /// A situational policy (DEC-077 §3): offered only while the town's
    /// situation calls for it.
    pub situational: bool,
    /// On the table now (always true for a policy that is not situational).
    pub available: bool,
    /// Fluent key of the circumstances a situational policy answers.
    pub unlock_reason_key: Option<String>,
    /// What the player knows of a situational policy (DEC-091; `None` for
    /// the standard menu, which is never gated). Below `known` the entry is
    /// a stub: its name, reason and lead, with no parameters, cost or side
    /// effects, and it cannot be drafted from here.
    pub knowledge: Option<crate::knowledge::KnowledgeView>,
}

/// What a draft with these parameters would cost and do, computed by the
/// engine for the catalogue's sliders (`QueryDto::PolicyPreview`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PolicyPreviewView {
    pub template: String,
    /// The parameters as the engine reads them (clamped, snapped to the step).
    pub params: Vec<ParamValue>,
    /// Estimated annual cost for the player's town (nominal currency).
    pub est_annual_cost: f64,
    /// That cost as a share of the town's annual budget.
    pub budget_share: f64,
    pub side_effects: Vec<SideEffectView>,
    pub support: SupportView,
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WhipView {
    pub politician: u32,
    pub name: String,
    pub party: Option<u32>,
    pub ward_number: u32,
    /// `yes` | `lean_yes` | `undecided` | `lean_no` | `no`
    pub lean: String,
    pub yes_probability: f64,
    pub lobby_odds: f64,
    pub lobby_odds_late: f64,
    pub is_player: bool,
    /// What the player knows of this member (`name` is empty below `heard`).
    pub level: crate::knowledge::KnowledgeLevelDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VoteView {
    pub date: DateView,
    pub stage: u32,
    pub yes: u32,
    pub no: u32,
    pub passed: bool,
    pub veto: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClauseDto {
    Earmark { ward: u32 },
    Sunset { years: u32 },
    Pilot { neighborhood: u32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParamValue {
    pub id: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LedgerEntry {
    /// Fluent key suffix, e.g. `homes_built` → `ledger-homes_built`.
    pub key: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BillView {
    pub id: u32,
    pub template: String,
    pub name_key: String,
    /// `draft` | `under_consideration` | `enacted` | `in_effect` | `ended`
    pub status: String,
    /// `defeated` | `vetoed` | `withdrawn` | `repealed` | `expired`
    pub end_reason: Option<String>,
    pub params: Vec<ParamValue>,
    pub clauses: Vec<ClauseDto>,
    pub sponsor_is_player: bool,
    /// Empty when the player has not heard of the sponsor.
    pub sponsor_name: String,
    pub next_vote: Option<DateView>,
    /// Fluent key of the next stage, e.g. `stage-vote-council`.
    pub next_stage_key: Option<String>,
    pub est_annual_cost: f64,
    pub funded_pct: f64,
    pub progress: f64,
    pub awareness: f64,
    /// Member by member, with yes chances and lobbying odds: only in the
    /// council chamber (DEC-079 rule 4). Elsewhere it is empty and
    /// `expected_yes` is rounded ("about N of M").
    pub whip: Vec<WhipView>,
    pub whip_exact: bool,
    pub expected_yes: f64,
    pub seats: u32,
    pub needed: u32,
    /// Who signs or vetoes the bill (the office of its procedure's `sign`
    /// stage, DEC-098) and how they lean; `None` when no office signs.
    pub signer: Option<SignerView>,
    pub votes: Vec<VoteView>,
    pub ledger: Vec<LedgerEntry>,
    pub support: SupportView,
    /// The player's declared vote: `yes` | `no` | `abstain`.
    pub my_vote: Option<String>,
    /// Whether the player may declare a vote now.
    pub can_vote: bool,
    /// The party line for the player: `yes` | `no` | `free`.
    pub party_line: String,
    /// Support for the bill among the player's ward voters (−1..1): to 0.25
    /// below `known` (`support.level`), 0.1 at `known`, exact in detail.
    pub ward_support: f64,
    /// The player owes the sponsor a favour.
    pub owe_sponsor: bool,
    pub side_effects: Vec<SideEffectView>,
}

/// The office that signs or vetoes a bill, and its holder's inclination.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SignerView {
    pub office: crate::post::PostDto,
    /// `sign` | `lean_sign` | `undecided` | `lean_veto` | `veto` (Fluent
    /// `signer-<lean>`).
    pub lean: String,
}

/// A body of a jurisdiction (DEC-017, DEC-098) and its members, in seat
/// order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BodyView {
    /// The body's constitution id.
    pub id: String,
    /// The constitution's Fluent key for the body (`name_key`).
    pub label_key: String,
    /// Fluent key of a member's title: `role-<member_key>`.
    pub role_key: String,
    pub at: crate::post::JurisdictionDto,
    /// The jurisdiction's legislature: the body of its `ordinance`
    /// procedure's first vote (bills are whipped there).
    pub legislature: bool,
    pub members: Vec<MemberView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MemberView {
    pub id: u32,
    pub name: String,
    pub party: Option<u32>,
    pub role_key: String,
    pub ward_number: Option<u32>,
    pub is_player: bool,
    /// Their character (open their sheet with `QueryDto::Character`).
    pub character: u32,
    /// Their opinion of the player (−100..100; DEC-031 relationship).
    pub opinion_of_player: f64,
    /// Favours: positive = they owe the player; negative = the player owes them.
    pub favors: i32,
    /// The member's two strongest convictions (attitude keys with sign),
    /// once the player knows them (DEC-091; empty below `known`).
    pub convictions: Vec<String>,
    /// How readily they trade votes, in detail only.
    pub pragmatism: Option<f64>,
    /// What the player knows of this member (`name` is empty below `heard`;
    /// role and ward are public).
    pub level: crate::knowledge::KnowledgeLevelDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlaceRef {
    pub id: u32,
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConcernView {
    /// Concern content id, e.g. `concern.jobs` (DEC-078).
    pub id: String,
    /// Fluent key of its name, e.g. `concern-jobs`.
    pub name_key: String,
    /// How strongly it is felt (0..1.5; its level in the neighbourhoods in
    /// question, from their own conditions, adjusted for how their residents
    /// lean).
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownView {
    /// Precise figures (at the town hall's statistics desk); otherwise the
    /// opinion figures are rounded to 5 points and concerns to 10 (DEC-079).
    pub detailed: bool,
    pub gov_approval: f64,
    pub player_awareness_ward: f64,
    pub player_approval_ward: f64,
    pub player_awareness_town: f64,
    pub player_approval_town: f64,
    /// As known (DEC-091): none, the top one, the top three to 0.1, or all
    /// (exact only at the statistics desk: the stricter of place and
    /// knowledge wins).
    pub concerns: Vec<ConcernView>,
    pub concerns_knowledge: crate::knowledge::KnowledgeView,
    pub neighborhoods: Vec<PlaceRef>,
    pub wards: Vec<PlaceRef>,
    /// Chance of a hostile town hall, aligned with `neighborhoods`.
    pub town_hall_risk: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OfficeView {
    pub player: PlayerView,
    pub catalog: Vec<TemplateView>,
    pub bills: Vec<BillView>,
    /// The bodies of the player's focus jurisdictions, in constitution
    /// order, each with its members (DEC-098).
    pub bodies: Vec<BodyView>,
    /// The holder of the executive office (the office of the `ordinance`
    /// procedure's `sign` stage) of the player's home jurisdiction.
    pub executive: Option<MemberView>,
    /// Deprecated (R2): the legislature's members (`bodies`); removed in
    /// the next release.
    pub council: Vec<MemberView>,
    /// Deprecated (R2): `executive`; removed in the next release.
    pub mayor: Option<MemberView>,
    pub town: TownView,
    pub date: DateView,
}

/// Round to a step: the pocket's coarse figures (DEC-079 rule 4).
pub fn coarse(v: f64, step: f64) -> f64 {
    (v / step).round() * step
}

pub(crate) fn snake<T: std::fmt::Debug>(v: &T) -> String {
    let s = format!("{v:?}");
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn full_name(p: &Politician) -> String {
    format!("{} {}", p.given, p.family)
}

fn ward_number(st: &State, w: ConstituencyId) -> u32 {
    st.world
        .constituencies
        .get(w.index())
        .map(|c| c.number)
        .unwrap_or(0)
}

fn lean_label(p: f64) -> &'static str {
    match p {
        x if x >= 0.85 => "yes",
        x if x >= 0.6 => "lean_yes",
        x if x > 0.4 => "undecided",
        x if x > 0.15 => "lean_no",
        _ => "no",
    }
}

fn member_view(gate: &Gate<'_>, engine: &Engine, p: &Politician) -> MemberView {
    let st = engine.state();
    let content = engine.content();
    let player = mandate_sim::characters::player_character(st);
    let level = gate.person(p.character);
    // Convictions iterate the attitude registry (DEC-075).
    let n = p.attitudes.len().min(content.attitudes.len());
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|a, b| {
        p.attitudes[*b]
            .abs()
            .total_cmp(&p.attitudes[*a].abs())
            .then(a.cmp(b))
    });
    let convictions = idx
        .iter()
        .take(if level >= Level::Known { 2 } else { 0 })
        .map(|&k| {
            format!(
                "{}{}",
                if p.attitudes[k] >= 0.0 {
                    "pro-"
                } else {
                    "anti-"
                },
                content.attitudes[k].key
            )
        })
        .collect();
    let role_key = crate::characters::label_role_key(engine, p.character);
    let ward = p.role.constituency().map(|w| ward_number(st, w));
    MemberView {
        id: p.id.0,
        name: gate.name(p.character, full_name(p)),
        party: p.party.map(|x| x.0),
        role_key,
        ward_number: ward,
        is_player: p.is_player,
        character: p.character.0,
        opinion_of_player: player
            .map(|me| mandate_sim::characters::opinion(&st.politics, p.character, me) as f64)
            .unwrap_or(0.0),
        favors: player
            .map(|me| mandate_sim::characters::favour_balance(&st.politics, p.character, me))
            .unwrap_or(0),
        convictions,
        pragmatism: (level >= Level::Detailed).then_some(p.pragmatism as f64),
        level: level.into(),
    }
}

/// A template's support in a town, as the player knows it (DEC-091).
fn support_view(
    gate: &Gate<'_>,
    t: &mandate_content::PolicyTemplate,
    st: &State,
    town: MunicipalityId,
    hoods: &[usize],
) -> SupportView {
    gate_support(
        gate.support(town, &t.id),
        support_by_class(t, &st.population, hoods),
        public_support(t, &st.population, hoods),
    )
}

pub(crate) fn bill_view(engine: &Engine, p: &PolicyInstance) -> Option<BillView> {
    let st = engine.state();
    let content = engine.content();
    let gate = Gate::of(engine);
    let town = MunicipalityId(p.jurisdiction);
    let resolved = mandate_sim::policy::resolve(content, &st.policy, p)?;
    let t = resolved.template();
    let hoods: Vec<usize> = st.world.hoods_of(MunicipalityId(p.jurisdiction)).collect();
    let pending = p.status == PolicyStatus::UnderConsideration || p.status == PolicyStatus::Draft;
    let constitution = content.constitutions.get(&st.meta.constitution);
    let leans = match (pending, constitution) {
        (true, Some(c)) => legislature_leans(
            &content.council,
            c,
            &st.world,
            &st.population,
            &st.politics,
            st.meta.seed,
            t,
            p,
        ),
        _ => Vec::new(),
    };
    let whip: Vec<WhipView> = leans
        .iter()
        .filter_map(|(id, lean)| {
            let m = st.politics.get(*id)?;
            let prob = yes_probability(&content.council, *lean);
            Some(WhipView {
                politician: id.0,
                name: gate.name(m.character, full_name(m)),
                level: gate.person(m.character).into(),
                party: m.party.map(|x| x.0),
                ward_number: m
                    .role
                    .constituency()
                    .map(|w| ward_number(st, w))
                    .unwrap_or(0),
                lean: lean_label(prob).into(),
                yes_probability: prob,
                lobby_odds: if m.is_player {
                    0.0
                } else {
                    lobby_odds(st, content, *id, false)
                },
                lobby_odds_late: if m.is_player {
                    0.0
                } else {
                    lobby_odds(st, content, *id, true)
                },
                is_player: m.is_player,
            })
        })
        .collect();
    let seats = whip.len() as u32;
    let expected_yes: f64 = whip.iter().map(|w| w.yes_probability).sum();
    let whip_exact = mandate_sim::places::player_has_service(st, content, "whip_exact");
    let stages = constitution
        .and_then(|c| c.tier(p.tier))
        .and_then(|t| t.procedure("ordinance"))
        .map(|p| p.stages.clone())
        .unwrap_or_default();
    let next_stage_key = if p.status == PolicyStatus::UnderConsideration {
        stages.get(p.stage as usize).map(|s| match s {
            mandate_content::StageDef::Vote { body, .. } => format!("stage-vote-{body}"),
            mandate_content::StageDef::Sign { office, .. } => format!("stage-sign-{office}"),
        })
    } else {
        None
    };
    // The office of the procedure's `sign` stage for the bill's
    // jurisdiction, with its holder's inclination (certain once decided).
    let signer = constitution
        .and_then(|c| mandate_sim::politics::executive_post(c, p.at()))
        .map(|office| {
            let chance = match (pending, constitution) {
                (true, Some(c)) => sign_probability(
                    &content.council,
                    executive_lean(
                        &content.council,
                        c,
                        &st.world,
                        &st.population,
                        &st.politics,
                        st.meta.seed,
                        t,
                        p,
                    ),
                ),
                (true, None) => sign_probability(&content.council, 0.0),
                (false, _) => 1.0,
            };
            SignerView {
                office: crate::post::post_dto(engine, office),
                lean: match chance {
                    x if x >= 0.85 => "sign",
                    x if x >= 0.6 => "lean_sign",
                    x if x > 0.4 => "undecided",
                    x if x > 0.15 => "lean_veto",
                    _ => "veto",
                }
                .into(),
            }
        });
    let sponsor = p.sponsor.and_then(|s| st.politics.get(s));
    let me = st.player.as_ref();
    let my_vote = me
        .and_then(|pl| st.politics.declared_votes.get(&(pl.politician.0, p.id.0)))
        .map(|d| {
            match d {
                1 => "yes",
                -1 => "no",
                _ => "abstain",
            }
            .to_string()
        });
    let powers = player_powers(st, content);
    let can_vote = p.status == PolicyStatus::UnderConsideration
        && p.sponsor != me.map(|pl| pl.politician)
        && p.jurisdiction == me.map(|pl| pl.municipality.0).unwrap_or(u32::MAX)
        && my_vote.is_none()
        && (powers.contains(&mandate_content::Power::ApproveOrdinance)
            || powers.contains(&mandate_content::Power::Veto));
    let ward_support = me
        .map(|pl| {
            gate_ward_support(
                gate.support(town, &t.id),
                public_support(
                    t,
                    &st.population,
                    &mandate_sim::policy::ward_hoods(&st.world, pl.ward),
                ),
            )
        })
        .unwrap_or(0.0);
    let owe_sponsor = p.sponsor.and_then(|s| st.politics.get(s)).is_some_and(|s| {
        !s.is_player
            && mandate_sim::characters::player_character(st).is_some_and(|me| {
                mandate_sim::characters::favours_owed(&st.politics, me, s.character) > 0
            })
    });
    Some(BillView {
        side_effects: side_effects_view(t, p),
        my_vote,
        can_vote,
        party_line: match mandate_sim::actions::party_line(st, p.id.0) {
            1 => "yes",
            -1 => "no",
            _ => "free",
        }
        .into(),
        ward_support,
        owe_sponsor,
        id: p.id.0,
        template: p.template.clone(),
        name_key: t.name_key.clone(),
        status: snake(&p.status),
        end_reason: p.end_reason.map(|r| snake(&r)),
        params: p
            .params
            .iter()
            .map(|(k, v)| ParamValue {
                id: k.clone(),
                value: *v,
            })
            .collect(),
        clauses: p
            .clauses
            .iter()
            .filter_map(|c| {
                Some(match c {
                    Clause::Earmark { ward } => ClauseDto::Earmark { ward: ward.0 },
                    Clause::Sunset { years } => ClauseDto::Sunset {
                        years: *years as u32,
                    },
                    Clause::Pilot { neighborhood } => ClauseDto::Pilot {
                        neighborhood: *neighborhood,
                    },
                    // A family bill's area follows its choices (its view is F3's).
                    Clause::Area { .. } => return None,
                })
            })
            .collect(),
        sponsor_is_player: sponsor.is_some_and(|s| s.is_player),
        sponsor_name: sponsor
            .map(|s| gate.name(s.character, full_name(s)))
            .unwrap_or_default(),
        next_vote: p.next_stage_on.map(DateView::of),
        next_stage_key,
        est_annual_cost: mandate_sim::actions::estimate_cost(st, content, p),
        funded_pct: st.finance.funded_share(p.id.0) * 100.0,
        progress: p.progress as f64,
        awareness: p.awareness as f64,
        expected_yes: if whip_exact {
            expected_yes
        } else {
            expected_yes.round()
        },
        seats,
        needed: seats / 2 + 1,
        whip: if whip_exact { whip } else { Vec::new() },
        whip_exact,
        signer,
        votes: p
            .votes
            .iter()
            .map(|v| VoteView {
                date: DateView::of(v.date),
                stage: v.stage as u32,
                yes: v.yes,
                no: v.no,
                passed: v.passed,
                veto: v.veto,
            })
            .collect(),
        ledger: p
            .ledger
            .iter()
            .map(|(k, v)| LedgerEntry {
                key: k.clone(),
                value: *v,
            })
            .collect(),
        support: support_view(&gate, t, st, town, &hoods),
    })
}

/// The engine's estimate for a draft of `template` with these parameters in
/// the player's town: cost, share of the budget, side effects, and support.
pub fn policy_preview(
    engine: &Engine,
    template: &str,
    params: &[ParamValue],
) -> Option<PolicyPreviewView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let t = content.policies.get(template)?;
    let m = pl.municipality;
    let given: std::collections::BTreeMap<String, f64> =
        params.iter().map(|p| (p.id.clone(), p.value)).collect();
    let probe = mandate_sim::policy::probe(t, m.0, &given);
    let residents = st.population.municipality_total(&st.world, m) as f64;
    let price = st.economy.national.price_level;
    let cost = mandate_sim::policy::annual_cost(t, &probe, residents, price);
    let budget = st
        .finance
        .local
        .get(m.index())
        .map(|x| x.budget_total().units() as f64)
        .unwrap_or(0.0);
    let hoods: Vec<usize> = st.world.hoods_of(m).collect();
    Some(PolicyPreviewView {
        template: t.id.clone(),
        params: t
            .params
            .iter()
            .map(|p| ParamValue {
                id: p.id.clone(),
                value: probe.param(&p.id),
            })
            .collect(),
        est_annual_cost: cost,
        budget_share: if budget > 0.0 { cost / budget } else { 0.0 },
        side_effects: side_effects_view(t, &probe),
        support: support_view(&Gate::of(engine), t, st, m, &hoods),
        available: mandate_sim::policy::unlocked(st, content, t, m.0),
    })
}

fn weighted<F: Fn(&mandate_sim::population::Cohort) -> f64>(
    st: &State,
    hoods: &[usize],
    f: F,
) -> f64 {
    let mut acc = 0.0;
    let mut w = 0.0;
    for &h in hoods {
        for c in &st.population.hoods[h].cohorts {
            let weight = c.count as f64;
            acc += f(c) * weight;
            w += weight;
        }
    }
    if w > 0.0 { acc / w } else { 0.0 }
}

pub fn office_view(engine: &Engine) -> Option<OfficeView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let me = st.politics.get(pl.politician)?;
    let m = pl.municipality;
    let mu = st.world.municipalities.get(m.index())?;
    let hoods: Vec<usize> = st.world.hoods_of(m).collect();
    let ward_hoods = mandate_sim::policy::ward_hoods(&st.world, pl.ward);
    let residents = st.population.municipality_total(&st.world, m) as f64;
    let price = st.economy.national.price_level;
    let gate = Gate::of(engine);
    let catalog = content
        .policies
        .values()
        .filter(|t| t.tier == TierKey::Local)
        .filter_map(|t| {
            // Situational policies are learned (DEC-091): omitted while
            // unknown, a stub once heard of, in full once known.
            let knowledge = t.unlock.as_ref().map(|_| {
                gate.view(&Subject::Situational {
                    town: m,
                    template: t.id.clone(),
                })
            });
            let level = knowledge.as_ref().map(|k| k.level);
            if level == Some(KnowledgeLevelDto::Unknown) {
                return None;
            }
            let stub = level == Some(KnowledgeLevelDto::Heard);
            let probe = mandate_sim::policy::probe(t, m.0, &Default::default());
            Some(TemplateView {
                id: t.id.clone(),
                name_key: t.name_key.clone(),
                desc_key: t.desc_key.clone(),
                category: snake(&t.category),
                params: if stub {
                    Vec::new()
                } else {
                    t.params.iter().map(param_view).collect()
                },
                implementation_months: t.implementation_months as u32,
                secondary_keys: t.secondary_keys.clone(),
                support: support_view(&gate, t, st, m, &hoods),
                est_cost_default: if stub {
                    0.0
                } else {
                    mandate_sim::policy::annual_cost(t, &probe, residents, price)
                },
                side_effects: if stub {
                    Vec::new()
                } else {
                    side_effects_view(t, &probe)
                },
                situational: t.unlock.is_some(),
                available: mandate_sim::policy::unlocked(st, content, t, m.0),
                unlock_reason_key: t.unlock.as_ref().map(|u| u.reason_key.clone()),
                knowledge,
            })
        })
        .collect();
    let mut bills: Vec<BillView> = st
        .policy
        .in_municipality(m.0)
        .filter_map(|p| bill_view(engine, p))
        .collect();
    bills.sort_by_key(|b| (b.status == "ended", std::cmp::Reverse(b.id)));
    let constitution = content.constitutions.get(&st.meta.constitution);
    let at = mandate_sim::world::Jurisdiction::town(m);
    let detailed = mandate_sim::places::player_has_service(st, content, "town_data_detailed");
    let concerns_subject = Subject::TownConcerns { town: m };
    let concerns = gate_concerns(
        gate.level(&concerns_subject),
        concerns_for(st, content, m, &hoods),
        detailed,
        usize::MAX,
    );
    let fine = |v: f64| if detailed { v } else { coarse(v, 0.05) };
    let can_propose =
        player_powers(st, content).contains(&mandate_content::Power::ProposeOrdinance);
    let posts: Vec<crate::post::PostDto> = mandate_sim::player::posts(st)
        .into_iter()
        .map(|p| crate::post::post_dto(engine, p))
        .collect();
    let bodies = bodies_view(engine, &gate);
    let executive = constitution
        .and_then(|c| mandate_sim::politics::executive_of(&st.politics, c, at))
        .and_then(|id| st.politics.get(id))
        .map(|p| member_view(&gate, engine, p));
    let legislature: Vec<MemberView> = bodies
        .iter()
        .find(|b| b.legislature && b.at == crate::post::jurisdiction_dto(&st.world, at))
        .map(|b| b.members.clone())
        .unwrap_or_default();
    Some(OfficeView {
        player: PlayerView {
            politician: me.id.0,
            name: full_name(me),
            party: me.party.map(|x| x.0),
            role_key: posts
                .first()
                .map(|p| p.role_key.clone())
                .unwrap_or_else(|| "role-private".into()),
            posts,
            municipality: mu.name.clone(),
            municipality_id: m.0,
            ward_number: ward_number(st, pl.ward),
            ward_id: pl.ward.0,
            slots_total: pl.standard_slots as u32,
            slots_left: pl.slots_left() as u32,
            late_night_available: !pl.late_night_used,
            mandate: pl.mandate as f64,
            money: pl.money.units() as f64,
            staff: pl.staff as u32,
            tracks: TracksView {
                viability: pl.tracks.viability as f64,
                legal_jeopardy: pl.tracks.legal_jeopardy as f64,
                threat: pl.tracks.threat as f64,
                standing: pl.tracks.standing as f64,
                health: pl.tracks.health as f64,
            },
            can_propose,
            slot_cap: pl.slot_cap() as u32,
            press_gaffe_risk: mandate_sim::actions::press_gaffe_risk(st, content),
            ads_left: mandate_sim::actions::MAX_ADS_PER_TURN.saturating_sub(pl.ads_this_turn)
                as u32,
            ad_cost: content.politics.campaign.ad_cost as f64,
            record: me.record as f64 * 100.0,
            renomination_standing: content.politics.vote.renomination_standing,
            shady: crate::media::shady_offers(engine),
            action_costs: content
                .tuning
                .action_costs
                .iter()
                .map(|(k, c)| ActionCostView {
                    action: k.clone(),
                    slots: c.slots as u32,
                    health: c.health,
                })
                .collect(),
            recent_actions: pl
                .actions
                .iter()
                .rev()
                .take(8)
                .map(|a| ActionLogView {
                    date: DateView::of(a.date),
                    action_key: a.action.clone(),
                    outcome_key: a.outcome.clone(),
                    args: a.args.iter().cloned().collect(),
                })
                .collect(),
        },
        catalog,
        bills,
        bodies,
        mayor: executive.clone(),
        executive,
        council: legislature,
        town: TownView {
            detailed,
            gov_approval: fine(weighted(st, &hoods, |c| c.gov_approval as f64)),
            player_awareness_ward: fine(weighted(st, &ward_hoods, |c| c.player_awareness as f64)),
            player_approval_ward: fine(weighted(st, &ward_hoods, |c| c.player_approval as f64)),
            player_awareness_town: fine(weighted(st, &hoods, |c| c.player_awareness as f64)),
            player_approval_town: fine(weighted(st, &hoods, |c| c.player_approval as f64)),
            concerns,
            concerns_knowledge: gate.view(&concerns_subject),
            neighborhoods: mu
                .neighborhoods
                .iter()
                .map(|h| {
                    let n = &st.world.neighborhoods[h.index()];
                    PlaceRef {
                        id: h.0,
                        name: n.name.clone(),
                        kind: snake(&n.kind),
                    }
                })
                .collect(),
            wards: st
                .world
                .constituencies
                .iter()
                .filter(|c| c.tier == mandate_sim::world::Tier::Local && c.jurisdiction == m.0)
                .map(|c| PlaceRef {
                    id: c.id.0,
                    name: c.number.to_string(),
                    kind: "ward".into(),
                })
                .collect(),
            town_hall_risk: mu
                .neighborhoods
                .iter()
                .map(|h| mandate_sim::actions::town_hall_risk(st, content, h.0))
                .collect(),
        },
        date: DateView::of(st.calendar.date),
    })
}

/// The bodies of the player's focus jurisdictions (DEC-098), in
/// constitution order, with their members in seat order.
fn bodies_view(engine: &Engine, gate: &Gate<'_>) -> Vec<BodyView> {
    let st = engine.state();
    let Some(c) = crate::post::constitution(engine) else {
        return Vec::new();
    };
    st.focus()
        .into_iter()
        .flat_map(|at| {
            let tier = c.tier(at.tier());
            let legislature = mandate_sim::politics::legislature(c, at.tier()).map(|b| &b.id);
            tier.into_iter()
                .flat_map(|t| t.bodies.iter())
                .filter(move |b| {
                    st.politics
                        .bodies
                        .iter()
                        .any(|x| x.at == at && x.body == b.id)
                })
                .map(move |b| BodyView {
                    id: b.id.clone(),
                    label_key: b.name_key.clone(),
                    role_key: format!("role-{}", b.member_key),
                    at: crate::post::jurisdiction_dto(&st.world, at),
                    legislature: legislature == Some(&b.id),
                    members: mandate_sim::politics::members_of(&st.politics, &b.id, at)
                        .iter()
                        .filter_map(|id| st.politics.get(*id))
                        .map(|p| member_view(gate, engine, p))
                        .collect(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// What worries residents of some neighbourhoods of a town, strongest first:
/// the engine's concern levels in those neighbourhoods (DEC-078 decision 3;
/// each from its own conditions, weighted by residents), as felt by those
/// residents (`mandate_sim::concerns::felt_concerns`). Concerns at level 0
/// are left out.
pub fn concerns_for(
    st: &State,
    content: &mandate_content::ContentBundle,
    m: MunicipalityId,
    hoods: &[usize],
) -> Vec<ConcernView> {
    mandate_sim::concerns::felt_concerns(&st.population, content, m.index(), hoods)
        .into_iter()
        .filter(|(_, w)| *w > 0.0)
        .filter_map(|(i, w)| {
            let c = content.concerns.get(i)?;
            Some(ConcernView {
                id: c.id.clone(),
                name_key: c.name_key.clone(),
                weight: w,
            })
        })
        .collect()
}

/// A town's strongest concerns by level (at most `n`; its neighbourhoods'
/// levels weighted by residents), for any town.
pub fn town_concerns(engine: &Engine, m: MunicipalityId, n: usize) -> Vec<ConcernView> {
    let content = engine.content();
    mandate_sim::concerns::top_concerns(&engine.state().population, m.index())
        .into_iter()
        .filter(|(_, w)| *w > 0.0)
        .take(n)
        .filter_map(|(i, w)| {
            let c = content.concerns.get(i)?;
            Some(ConcernView {
                id: c.id.clone(),
                name_key: c.name_key.clone(),
                weight: w,
            })
        })
        .collect()
}

fn param_view(p: &ParamDef) -> ParamView {
    ParamView {
        id: p.id.clone(),
        name_key: p.name_key.clone(),
        unit_key: p.unit_key.clone(),
        min: p.min,
        max: p.max,
        step: p.step,
        default: p.default,
    }
}
