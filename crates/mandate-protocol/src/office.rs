//! Office views (M3): the player's desk, bills with whip counts, the council,
//! the policy catalogue, and town opinion. Estimates shown here are what the
//! player can know (whip counts are probabilities, REQ-013).

use crate::views::DateView;
use mandate_content::{ParamDef, TierKey};
use mandate_sim::actions::{lobby_odds, player_powers};
use mandate_sim::policy::{
    Clause, PolicyInstance, PolicyStatus, council_leans, mayor_lean, public_support,
    sign_probability, support_by_class, yes_probability,
};
use mandate_sim::politics::{Politician, Role};
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
    /// Fluent key, e.g. `role-councillor`.
    pub role_key: String,
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
    /// −1..1 by class and overall (turnout-weighted).
    pub low: f64,
    pub middle: f64,
    pub high: f64,
    pub overall: f64,
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
    /// Mayor's inclination: `sign` | `lean_sign` | `undecided` | `lean_veto` | `veto`.
    pub mayor: String,
    pub votes: Vec<VoteView>,
    pub ledger: Vec<LedgerEntry>,
    pub support: SupportView,
    /// The player's declared vote: `yes` | `no` | `abstain`.
    pub my_vote: Option<String>,
    /// Whether the player may declare a vote now.
    pub can_vote: bool,
    /// The party line for the player: `yes` | `no` | `free`.
    pub party_line: String,
    /// Support for the bill among the player's ward voters (−1..1).
    pub ward_support: f64,
    /// The player owes the sponsor a favour.
    pub owe_sponsor: bool,
    pub side_effects: Vec<SideEffectView>,
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
    /// The member's two strongest convictions (attitude keys with sign).
    pub convictions: Vec<String>,
    pub pragmatism: f64,
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
    /// How strongly it is felt (0..1.5; the town's level, adjusted for how
    /// the residents in question lean).
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
    pub concerns: Vec<ConcernView>,
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
    pub council: Vec<MemberView>,
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

fn member_view(st: &State, content: &mandate_content::ContentBundle, p: &Politician) -> MemberView {
    let player = mandate_sim::characters::player_character(st);
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
        .take(2)
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
    let (role_key, ward) = match p.role {
        Role::Councillor { ward, .. } => ("role-councillor", Some(ward_number(st, ward))),
        Role::Mayor { .. } => ("role-mayor", None),
        Role::Private { .. } => ("role-private", None),
    };
    MemberView {
        id: p.id.0,
        name: full_name(p),
        party: p.party.map(|x| x.0),
        role_key: role_key.into(),
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
        pragmatism: p.pragmatism as f64,
    }
}

fn support_view(c: [f64; 3], overall: f64) -> SupportView {
    SupportView {
        low: c[0],
        middle: c[1],
        high: c[2],
        overall,
    }
}

pub(crate) fn bill_view(engine: &Engine, p: &PolicyInstance) -> Option<BillView> {
    let st = engine.state();
    let content = engine.content();
    let t = content.policies.get(&p.template)?;
    let hoods: Vec<usize> = st.world.hoods_of(MunicipalityId(p.jurisdiction)).collect();
    let pending = p.status == PolicyStatus::UnderConsideration || p.status == PolicyStatus::Draft;
    let leans = if pending {
        council_leans(
            &content.council,
            &st.world,
            &st.population,
            &st.politics,
            st.meta.seed,
            t,
            p,
        )
    } else {
        Vec::new()
    };
    let whip: Vec<WhipView> = leans
        .iter()
        .filter_map(|(id, lean)| {
            let m = st.politics.get(*id)?;
            let prob = yes_probability(&content.council, *lean);
            Some(WhipView {
                politician: id.0,
                name: full_name(m),
                party: m.party.map(|x| x.0),
                ward_number: match m.role {
                    Role::Councillor { ward, .. } => ward_number(st, ward),
                    Role::Mayor { .. } | Role::Private { .. } => 0,
                },
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
    let constitution = content.constitutions.get(&st.meta.constitution);
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
    let mayor = if pending {
        sign_probability(
            &content.council,
            mayor_lean(
                &content.council,
                &st.world,
                &st.population,
                &st.politics,
                st.meta.seed,
                t,
                p,
            ),
        )
    } else {
        1.0
    };
    let mayor_label = match mayor {
        x if x >= 0.85 => "sign",
        x if x >= 0.6 => "lean_sign",
        x if x > 0.4 => "undecided",
        x if x > 0.15 => "lean_veto",
        _ => "veto",
    };
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
            public_support(
                t,
                &st.population,
                &mandate_sim::policy::ward_hoods(&st.world, pl.ward),
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
            .map(|c| match c {
                Clause::Earmark { ward } => ClauseDto::Earmark { ward: ward.0 },
                Clause::Sunset { years } => ClauseDto::Sunset {
                    years: *years as u32,
                },
                Clause::Pilot { neighborhood } => ClauseDto::Pilot {
                    neighborhood: *neighborhood,
                },
            })
            .collect(),
        sponsor_is_player: sponsor.is_some_and(|s| s.is_player),
        sponsor_name: sponsor.map(full_name).unwrap_or_default(),
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
        mayor: mayor_label.into(),
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
        support: support_view(
            support_by_class(t, &st.population, &hoods),
            public_support(t, &st.population, &hoods),
        ),
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
    let catalog = content
        .policies
        .values()
        .filter(|t| t.tier == TierKey::Local)
        .map(|t| {
            let defaults: std::collections::BTreeMap<String, f64> =
                t.params.iter().map(|p| (p.id.clone(), p.default)).collect();
            let probe = PolicyInstance {
                id: mandate_sim::policy::PolicyInstanceId(u32::MAX),
                template: t.id.clone(),
                tier: TierKey::Local,
                jurisdiction: m.0,
                sponsor: None,
                params: defaults,
                clauses: Vec::new(),
                status: PolicyStatus::Draft,
                stage: 0,
                next_stage_on: None,
                votes: Vec::new(),
                introduced_on: None,
                enacted_on: None,
                progress: 0.0,
                awareness: 0.0,
                end_reason: None,
                cause: None,
                ledger: Default::default(),
            };
            TemplateView {
                id: t.id.clone(),
                name_key: t.name_key.clone(),
                desc_key: t.desc_key.clone(),
                category: snake(&t.category),
                params: t.params.iter().map(param_view).collect(),
                implementation_months: t.implementation_months as u32,
                secondary_keys: t.secondary_keys.clone(),
                support: support_view(
                    support_by_class(t, &st.population, &hoods),
                    public_support(t, &st.population, &hoods),
                ),
                est_cost_default: mandate_sim::policy::annual_cost(t, &probe, residents, price),
                side_effects: side_effects_view(t, &probe),
            }
        })
        .collect();
    let mut bills: Vec<BillView> = st
        .policy
        .in_municipality(m.0)
        .filter_map(|p| bill_view(engine, p))
        .collect();
    bills.sort_by_key(|b| (b.status == "ended", std::cmp::Reverse(b.id)));
    let council = st.politics.councils.get(m.index());
    let detailed = mandate_sim::places::player_has_service(st, content, "town_data_detailed");
    let concerns = concerns_for(st, content, m, &hoods);
    let fine = |v: f64| if detailed { v } else { coarse(v, 0.05) };
    let can_propose =
        player_powers(st, content).contains(&mandate_content::Power::ProposeOrdinance);
    Some(OfficeView {
        player: PlayerView {
            politician: me.id.0,
            name: full_name(me),
            party: me.party.map(|x| x.0),
            role_key: match me.role {
                Role::Councillor { .. } => "role-councillor",
                Role::Mayor { .. } => "role-mayor",
                Role::Private { .. } => "role-private",
            }
            .into(),
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
        council: council
            .map(|c| {
                c.members
                    .iter()
                    .filter_map(|id| st.politics.get(*id))
                    .map(|p| member_view(st, content, p))
                    .collect()
            })
            .unwrap_or_default(),
        mayor: council
            .and_then(|c| c.mayor)
            .and_then(|id| st.politics.get(id))
            .map(|p| member_view(st, content, p)),
        town: TownView {
            detailed,
            gov_approval: fine(weighted(st, &hoods, |c| c.gov_approval as f64)),
            player_awareness_ward: fine(weighted(st, &ward_hoods, |c| c.player_awareness as f64)),
            player_approval_ward: fine(weighted(st, &ward_hoods, |c| c.player_approval as f64)),
            player_awareness_town: fine(weighted(st, &hoods, |c| c.player_awareness as f64)),
            player_approval_town: fine(weighted(st, &hoods, |c| c.player_approval as f64)),
            concerns: concerns
                .into_iter()
                .map(|c| ConcernView {
                    weight: if detailed {
                        c.weight
                    } else {
                        coarse(c.weight, 0.1)
                    },
                    ..c
                })
                .collect(),
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

/// What worries residents of some neighbourhoods of a town, strongest first:
/// the engine's concern levels (DEC-078 decision 3), as felt by those
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

/// A town's strongest concerns by level (at most `n`), for any town.
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
