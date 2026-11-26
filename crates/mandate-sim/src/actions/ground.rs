//! The ground game: town halls and canvassing, and the campaign contact they make.

use super::*;

/// How well a canvass's worry fits the neighbourhood (DEC-091 "ignorance
/// costs effectiveness, never fairness"; M6_ENGINE_CONTRACTS §1.6): the
/// multiplier on its approval gain, `1 + match_bonus × (salience − 1)`
/// within [`floor`, `ceiling`] (`knowledge.effects.canvass`). Salience is the
/// neighbourhood's true felt level of that concern over the mean of its
/// felt concerns (1: an average worry, 0: one nobody feels here). No issue
/// (a general canvass) is 1. The truth decides; what the player knows only
/// decides which worry they can aim at. The command and any preview share
/// this function.
pub fn canvass_match(
    state: &State,
    content: &ContentBundle,
    hood: NeighborhoodId,
    issue: Option<&str>,
) -> f64 {
    let Some(issue) = issue else {
        return 1.0;
    };
    if state.world.neighborhoods.get(hood.index()).is_none() {
        return 1.0;
    }
    let felt = crate::concerns::ranked(
        &state.population,
        content,
        &state.world,
        crate::concerns::Scope::Neighbourhood(hood),
        mandate_content::condition::Worry::Felt,
    );
    let mean = felt.iter().map(|x| x.1).sum::<f64>() / felt.len().max(1) as f64;
    let mine = felt
        .iter()
        .find(|(i, _)| content.concerns.get(*i).is_some_and(|c| c.id == issue))
        .map(|x| x.1)
        .unwrap_or(0.0);
    let salience = if mean > 0.0 { mine / mean } else { 1.0 };
    let k = &content.knowledge.effects.canvass;
    clamp(1.0 + k.match_bonus * (salience - 1.0), k.floor, k.ceiling)
}

/// A canvass fit in words (`strong` | `fair` | `weak`), for the outcome.
fn fit_band(content: &ContentBundle, fit: f64) -> &'static str {
    let k = &content.knowledge.effects.canvass;
    if fit > 1.0 && fit >= 1.0 + 0.5 * (k.ceiling - 1.0) {
        "strong"
    } else if fit < 1.0 && fit <= 1.0 - 0.5 * (1.0 - k.floor) {
        "weak"
    } else {
        "fair"
    }
}

/// Chance a town hall turns hostile in a neighbourhood: likelier where the
/// player and the town hall are unpopular (shown before acting).
pub fn town_hall_risk(state: &State, content: &ContentBundle, hood: u32) -> f64 {
    let Some(h) = state.population.hoods.get(hood as usize) else {
        return 0.0;
    };
    let (mut ap, mut gov, mut n) = (0.0, 0.0, 0.0);
    for c in &h.cohorts {
        let w = c.count as f64;
        ap += c.player_approval as f64 * c.player_awareness as f64 * w;
        gov += c.gov_approval as f64 * w;
        n += w;
    }
    let ap = ap / n.max(1.0);
    let gov = gov / n.max(1.0);
    let t = &content.actions.town_hall;
    clamp(
        t.risk_base - t.risk_approval * ap - t.risk_government * (gov - 0.5),
        t.risk_min,
        t.risk_max,
    )
}

/// Apply campaign contact to the player's candidacy, if a race is running and
/// covers the neighbourhood (`None` = every neighbourhood in the race).
pub(super) fn campaign_contact(
    state: &mut State,
    content: &ContentBundle,
    hood: Option<u32>,
    scale: f64,
) -> bool {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return false;
    };
    let params = &content.politics.campaign;
    let mut touched = false;
    for race in state.politics.elections.races.iter_mut() {
        let Some(c) = race.candidates.iter_mut().find(|c| c.politician == me) else {
            continue;
        };
        for (i, h) in race.hoods.iter().enumerate() {
            if hood.is_none_or(|x| x == *h) {
                crate::politics::elections::contact(c, i, params, scale);
                touched = true;
            }
        }
    }
    touched
}

/// Canvassing a neighbourhood (DEC-091, §1.3): its worries `known` (a
/// second canvass reads them: `detailed`, dated), and one resident found on
/// its streets met. Nobody new is created: only people already stored.
fn learn_canvass(state: &mut State, content: &ContentBundle, hood: NeighborhoodId) {
    let src = knowledge::action_source(state, "canvass");
    knowledge::learn_hood(state, content, hood, src.clone());
    let me = characters::player_character(state);
    let Some(pl) = state.player.as_ref() else {
        return;
    };
    let resident = state.politics.characters.people.iter().find(|c| {
        Some(c.id) != me
            && c.named.as_deref().is_some_and(|n| n.home == hood.0)
            && pl
                .knowledge
                .level(&knowledge::Subject::Person { person: c.id })
                < Level::Known
    });
    if let Some(person) = resident.map(|c| c.id) {
        knowledge::teach_one(
            state,
            knowledge::Subject::Person { person },
            Level::Known,
            src,
        );
    }
}

/// Hold a town hall in a neighbourhood, optionally about a bill.
pub(super) fn town_hall(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
    m: u32,
    neighborhood: &u32,
    policy: &Option<u32>,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    let h = state
        .world
        .neighborhoods
        .get(*neighborhood as usize)
        .ok_or_else(|| bad("no such neighbourhood"))?;
    if h.municipality.0 != m {
        return Err(bad("that neighbourhood is not in your town"));
    }
    if let Some(id) = policy {
        local_policy(state, *id)?;
    }
    let risk = town_hall_risk(state, content, *neighborhood);
    spend(state, content, "town_hall", false)?;
    crate::politics::add_record(
        &mut state.politics,
        pl.politician,
        content.career.record_gain.town_hall as f32,
    );
    let fx = &content.actions.town_hall;
    let hostile = state.rng.get(Stream::Player).chance(risk);
    let oratory = state
        .politics
        .get(pl.politician)
        .map(|x| x.oratory)
        .unwrap_or(0.5) as f64;
    let sig = policy
        .and_then(|id| state.policy.instances.get(id as usize))
        .and_then(|p| crate::policy::resolve(content, &state.policy, p))
        .map(|t| t.signature_weights.clone());
    for c in state.population.hoods[*neighborhood as usize]
        .cohorts
        .iter_mut()
    {
        c.player_awareness = clamp(
            c.player_awareness as f64 + fx.awareness * (fx.awareness_oratory_base + oratory),
            0.0,
            1.0,
        ) as f32;
        let nudge = sig
            .as_ref()
            .map(|s| crate::politics::alignment(s, &c.attitudes) * fx.bill_nudge)
            .unwrap_or(fx.nudge);
        let room = 1.0 - (c.player_approval as f64).abs();
        // A hostile crowd: people still learn your name, but not kindly.
        let nudge = if hostile { fx.hostile_nudge } else { nudge };
        c.player_approval = clamp(c.player_approval as f64 + nudge * room, -1.0, 1.0) as f32;
    }
    if let Some(id) = policy
        && let Some(p) = state.policy.instances.get_mut(*id as usize)
    {
        p.awareness = clamp(
            p.awareness as f64 + fx.bill_awareness * (0.5 + oratory),
            0.0,
            1.0,
        ) as f32;
    }
    let scale = (fx.contact_base + fx.contact_oratory * oratory)
        * if hostile { fx.hostile_contact } else { 1.0 };
    campaign_contact(state, content, Some(*neighborhood), scale);
    // A meeting hears the neighbourhood out (DEC-091); with
    // a bill, who supports it.
    let src = knowledge::action_source(state, "town_hall");
    knowledge::learn_hood(state, content, NeighborhoodId(*neighborhood), src.clone());
    if let Some(t) = policy
        .and_then(|id| state.policy.instances.get(id as usize))
        .map(|p| p.template.clone())
    {
        knowledge::teach_one(
            state,
            knowledge::Subject::PolicySupport {
                town: MunicipalityId(m),
                policy: knowledge::PolicyRef::Template { template: t },
            },
            Level::Known,
            src,
        );
    }
    crate::asks::record_deed(
        state,
        crate::asks::Deed::TownHall {
            hood: NeighborhoodId(*neighborhood),
        },
    );
    res.outcome_key = if hostile {
        "outcome-town-hall-hostile"
    } else {
        "outcome-town-hall"
    }
    .into();
    res.args.insert(
        "place".into(),
        state.world.neighborhoods[*neighborhood as usize]
            .name
            .clone(),
    );
    Ok(res)
}

/// What the player says at a door (DEC-114 §3.3): one choice of three.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DoorChoice {
    /// Listen: they say what worries them. Doors of a street that name the
    /// same worry teach the neighbourhood's worries.
    Listen,
    /// Make the case on a worry (a concern id), as a canvass's issue: the
    /// worry argued at most doors of the street is the canvass's.
    Case { issue: String },
    /// Ask for their vote (in a campaign) or a poster in the window: a
    /// promise from them, kept on the record.
    Ask,
}

/// Whether a command goes on with the canvass under way: a door of its
/// street, or finishing it (`Canvass` on its neighbourhood). Anything else
/// ends the street first (`close_street`).
pub(super) fn goes_on_with_street(state: &State, cmd: &PlayerCommand) -> bool {
    let Some(st) = state.politics.characters.households.street.as_ref() else {
        return true;
    };
    match cmd {
        PlayerCommand::Act {
            action: Action::Doorstep { neighborhood, .. } | Action::Canvass { neighborhood, .. },
            late_night: false,
        } => *neighborhood == st.hood.0,
        _ => false,
    }
}

/// A neighbourhood of the player's home town, else why not.
fn home_hood(state: &State, neighborhood: u32) -> Result<NeighborhoodId, EngineError> {
    let h = state
        .world
        .neighborhoods
        .get(neighborhood as usize)
        .ok_or_else(|| bad("no such neighbourhood"))?;
    if Some(h.municipality) != crate::player::home(state) {
        return Err(bad("that neighbourhood is not in your town"));
    }
    Ok(NeighborhoodId(neighborhood))
}

/// Canvass a neighbourhood, optionally on one worry (DEC-097 §1, DEC-114
/// §3.3). With a street under way there, this finishes it: today's formula,
/// with the worry argued at most of its doors as the issue (`issue` is not
/// read). Otherwise it is the whole street in one tap: the engine chooses
/// the doors (`households::engine_doors`), makes the case on `issue` at
/// each (or listens, without one), and finishes. Either way the doors add
/// no approval beyond the formula: knowledge, memory and the doorstep ask.
pub(super) fn canvass(
    state: &mut State,
    content: &ContentBundle,
    _pl: &crate::player::PlayerState,
    neighborhood: &u32,
    issue: &Option<String>,
) -> Result<ActionResult, EngineError> {
    let hood = home_hood(state, *neighborhood)?;
    if state
        .politics
        .characters
        .households
        .street
        .as_ref()
        .is_some_and(|s| s.hood == hood)
    {
        return finish_street(state, content, None).ok_or_else(|| bad("no canvass under way"));
    }
    if let Some(i) = issue
        && !content.concerns.iter().any(|c| &c.id == i)
    {
        return Err(bad("no such worry"));
    }
    spend(state, content, "canvass", false)?;
    open_street(state, hood);
    let choice = match issue {
        Some(i) => DoorChoice::Case { issue: i.clone() },
        None => DoorChoice::Listen,
    };
    for door in characters::households::engine_doors(state, content, hood) {
        if let Some(h) = characters::households::household(state, content, hood, door) {
            knock(state, content, &h, &choice);
        }
    }
    // The one tap argues the player's worry, whoever was in.
    finish_street(state, content, Some(issue.clone())).ok_or_else(|| bad("no canvass under way"))
}

/// Knock at a door of the neighbourhood the player stands in (DEC-114
/// §3.3): the first knock of a street spends the canvass's time; the last
/// of `households.street_doors` finishes it (its outcome in the reply's
/// `street_*` arguments).
pub(super) fn doorstep(
    state: &mut State,
    content: &ContentBundle,
    _pl: &crate::player::PlayerState,
    neighborhood: u32,
    door: u16,
    choice: &DoorChoice,
) -> Result<ActionResult, EngineError> {
    let hood = home_hood(state, neighborhood)?;
    if let DoorChoice::Case { issue } = choice
        && !content.concerns.iter().any(|c| &c.id == issue)
    {
        return Err(bad("no such worry"));
    }
    let h = characters::households::household(state, content, hood, door)
        .ok_or_else(|| bad("no such door"))?;
    let open = state.politics.characters.households.street.as_ref();
    if open.is_some_and(|s| s.doors.contains(&door)) {
        return Err(bad("you have knocked there already"));
    }
    if open.is_none() {
        spend(state, content, "canvass", false)?;
        open_street(state, hood);
    }
    let mut res = knock(state, content, &h, choice);
    let knocked = state
        .politics
        .characters
        .households
        .street
        .as_ref()
        .map_or(0, |s| s.doors.len());
    if knocked >= content.characters.households.street_doors.max(1) as usize
        && let Some(fin) = finish_street(state, content, None)
    {
        log_result(state, &fin);
        res.args.insert("street".into(), "done".into());
        for (k, v) in fin.args {
            res.args.insert(format!("street_{k}"), v);
        }
    }
    Ok(res)
}

fn open_street(state: &mut State, hood: NeighborhoodId) {
    state.politics.characters.households.street = Some(characters::households::Street {
        hood,
        on: state.calendar.date,
        doors: Vec::new(),
        argued: Vec::new(),
        heard: Vec::new(),
    });
}

/// One doorstep exchange (§3.3): who answers, the player's choice, their
/// reply, and what is remembered. Adds no approval (the street's formula is
/// applied once, as it finishes).
fn knock(
    state: &mut State,
    content: &ContentBundle,
    h: &characters::households::Household,
    choice: &DoorChoice,
) -> ActionResult {
    use characters::households::{DoorPromise, DoorstepRecord, Said, Went};
    let p = &content.characters.households;
    let date = state.calendar.date;
    let mut res = ActionResult::default();
    res.args
        .insert("number".into(), (h.door as u32 + 1).to_string());
    res.args.insert("family".into(), h.family.clone());
    res.args.insert("given".into(), h.given.clone());
    if let Some(n) = state.world.neighborhoods.get(h.hood.index()) {
        res.args.insert("place".into(), n.name.clone());
    }
    let answers = characters::households::answers(state, content, h);
    let worry_id = |i: usize| content.concerns.get(i).map(|c| c.id.clone());
    let worry_key = |id: &str| {
        content
            .concerns
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.talk_key.clone().unwrap_or_else(|| c.name_key.clone()))
            .unwrap_or_default()
    };
    let (went, said, worry, promise) = if !answers {
        res.outcome_key = if h.vacant {
            "door-reply-vacant"
        } else {
            "door-reply-not-in"
        }
        .into();
        (Went::NotIn, Said::Knocked, None, None)
    } else {
        match choice {
            DoorChoice::Listen => {
                let w = h.worry.and_then(worry_id);
                res.outcome_key = if w.is_some() {
                    "door-reply-listen"
                } else {
                    "door-reply-listen-none"
                }
                .into();
                if let Some(w) = &w {
                    res.args.insert("worry_key".into(), worry_key(w));
                    res.args.insert("worry".into(), w.clone());
                    // Doors that name the same worry: the neighbourhood's
                    // worries are known (DEC-091), from the doorsteps.
                    let agreed = state
                        .politics
                        .characters
                        .households
                        .street
                        .as_mut()
                        .map(|s| {
                            s.heard.push(w.clone());
                            s.heard.iter().filter(|x| *x == w).count()
                        })
                        .unwrap_or(0);
                    if agreed >= p.agree.max(1) as usize
                        && knowledge::teach_one(
                            state,
                            knowledge::Subject::HoodConcerns { hood: h.hood },
                            Level::Known,
                            knowledge::action_source(state, "canvass"),
                        )
                    {
                        res.args.insert("agreed".into(), "1".into());
                    }
                }
                let went = if !h.aware || h.view > p.cool {
                    Went::Warm
                } else {
                    Went::Cool
                };
                (went, Said::Listened, w, None)
            }
            DoorChoice::Case { issue } => {
                // How it landed: their own worry lands best; else as the
                // neighbourhood's fit (the formula's measure).
                let landed = if h.worry.and_then(worry_id).as_deref() == Some(issue.as_str()) {
                    "strong"
                } else {
                    fit_band(content, canvass_match(state, content, h.hood, Some(issue)))
                };
                res.outcome_key = format!("door-reply-case-{landed}");
                res.args.insert("issue_key".into(), worry_key(issue));
                res.args.insert("issue".into(), issue.clone());
                if let Some(s) = state.politics.characters.households.street.as_mut() {
                    s.argued.push(issue.clone());
                }
                let went = match landed {
                    "strong" => Went::Warm,
                    "weak" => Went::Cool,
                    _ if h.view > p.cool => Went::Warm,
                    _ => Went::Cool,
                };
                (went, Said::MadeTheCase, Some(issue.clone()), None)
            }
            DoorChoice::Ask => {
                let campaign = state
                    .player
                    .as_ref()
                    .is_some_and(|pl| characters::is_candidate(&state.politics, pl.politician));
                let kind = if campaign {
                    DoorPromise::Vote
                } else {
                    DoorPromise::Poster
                };
                let yes = characters::households::ask_roll(state, h)
                    < characters::households::ask_chance(content, h);
                res.outcome_key = format!(
                    "door-reply-ask-{}-{}",
                    kind.key(),
                    if yes { "yes" } else { "no" }
                );
                let went = if yes { Went::Warm } else { Went::Cool };
                (went, Said::Asked, None, yes.then_some(kind))
            }
        }
    };
    if let Some(s) = state.politics.characters.households.street.as_mut() {
        s.doors.push(h.door);
    }
    res.args.insert("went".into(), went.key().into());
    // Nobody lives at an empty house: nothing to remember.
    if let (false, Some(by)) = (h.vacant, characters::player_character(state)) {
        state.politics.characters.households.remember(
            p,
            DoorstepRecord {
                by,
                hood: h.hood,
                door: h.door,
                generation: h.generation,
                on: date,
                went,
                said,
                worry,
                promise,
                speaker: h.speaker,
            },
        );
    }
    res
}

/// The canvass under way ends (`Canvass` on its street, its last door, any
/// other command, the turn's end): today's formula applies, with the worry
/// argued at most of its doors as the issue, and it is logged. Nothing
/// happens without one.
pub fn close_street(state: &mut State, content: &ContentBundle) {
    if let Some(res) = finish_street(state, content, None) {
        log_result(state, &res);
    }
}

fn log_result(state: &mut State, res: &ActionResult) {
    let date = state.calendar.date;
    if let Some(p) = state.player.as_mut() {
        p.log(
            date,
            "action-canvass",
            &res.outcome_key,
            res.args
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );
    }
}

/// Finish the street: the canvass formula (as one canvass always was), its
/// deed, and the doorstep ask's speaker. The issue is `issue` when given
/// (the one tap), else the street's. `None` without a street.
fn finish_street(
    state: &mut State,
    content: &ContentBundle,
    issue: Option<Option<String>>,
) -> Option<ActionResult> {
    let street = state.politics.characters.households.street.take()?;
    let politician = state.player.as_ref()?.politician;
    let issue = issue.unwrap_or_else(|| street.issue());
    let mut res = canvass_effect(state, content, politician, street.hood, issue.as_ref());
    res.args
        .insert("doors".into(), street.doors.len().to_string());
    // The deed, and the one doorstep roll a canvass makes (`asks::offers`):
    // when it brings an ask, it is from a household knocked on today, whose
    // speaker becomes a Named resident (§3.4).
    let date = state.calendar.date;
    let deed = crate::asks::Deed::Canvass { hood: street.hood };
    let cause = crate::spine::Cause::Player(deed.act());
    let ev = state
        .events
        .record(date, EventKind::PlayerDeed { deed }, cause);
    if crate::asks::offers::doorstep_due(state, content, ev.0) {
        speaker_from_street(state, content, &street, ev.0);
    }
    Some(res)
}

/// Promote the answerer of one household knocked on in a street (keyed by
/// the canvass's deed) and name them on its record.
fn speaker_from_street(
    state: &mut State,
    content: &ContentBundle,
    street: &characters::households::Street,
    event: u32,
) {
    let Some(me) = characters::player_character(state) else {
        return;
    };
    let date = state.calendar.date;
    let hh = &state.politics.characters.households;
    let answered: Vec<u16> = street
        .doors
        .iter()
        .copied()
        .filter(|d| {
            hh.record(me, street.hood, *d, 0)
                .is_some_and(|r| r.on == date && r.went != characters::households::Went::NotIn)
        })
        .collect();
    if answered.is_empty() {
        return;
    }
    let pick = mandate_core::rng::keyed(
        state.meta.seed,
        Stream::Npc,
        &[characters::households::KEY_SPEAKER, event as u64],
    )
    .below(answered.len() as u64) as usize;
    let Some(h) = characters::households::household(state, content, street.hood, answered[pick])
    else {
        return;
    };
    let Some(id) = characters::households::promote(state, content, &h) else {
        return;
    };
    if let Some(r) = state
        .politics
        .characters
        .households
        .records
        .iter_mut()
        .find(|r| {
            r.by == me && r.hood == h.hood && r.door == h.door && r.generation == h.generation
        })
    {
        r.speaker = Some(id);
    }
}

/// Today's canvass formula on a neighbourhood (DEC-097 §1), unchanged by
/// doors: awareness and approval of every cohort, campaign contact, the
/// player's memory of the doorsteps, what a canvass teaches.
fn canvass_effect(
    state: &mut State,
    content: &ContentBundle,
    politician: PoliticianId,
    hood: NeighborhoodId,
    issue: Option<&String>,
) -> ActionResult {
    let mut res = ActionResult::default();
    let fx = &content.actions.canvass;
    // Decided before the doorsteps change anything: how the
    // worry fits, and how fresh the doorsteps are (DEC-097 §1).
    let fit = canvass_match(state, content, hood, issue.map(|s| s.as_str()));
    let curve = crate::player::ground::canvass_curve(state, fx, hood);
    crate::politics::add_record(
        &mut state.politics,
        politician,
        content.career.record_gain.canvass as f32,
    );
    for c in state.population.hoods[hood.index()].cohorts.iter_mut() {
        // Contact builds recognition first; persuasion is
        // small, saturating, fades with repeated visits, and
        // counts for more in campaign season (DEC-071 §5,
        // DEC-097 §1).
        let aw = c.player_awareness as f64;
        c.player_awareness =
            clamp(aw + fx.awareness * (1.0 - aw) * curve.awareness, 0.0, 1.0) as f32;
        let room = 1.0 - (c.player_approval as f64).abs();
        c.player_approval = clamp(
            c.player_approval as f64 + fx.approval * fit * room * curve.approval,
            -1.0,
            1.0,
        ) as f32;
    }
    // A full day on the doorsteps: deeper than any other
    // contact (the race model has its own diminishing).
    campaign_contact(state, content, Some(hood.0), fx.contact * curve.same_turn);
    let (on, turn) = (state.calendar.date, state.calendar.turn);
    if let Some(p) = state.player.as_mut() {
        p.ground.remember(hood, curve.visits + 1.0, on, turn);
    }
    learn_canvass(state, content, hood);
    res.outcome_key = "outcome-canvass".into();
    // How fresh the doorsteps were (the player's own memory).
    res.args.insert(
        "fresh".into(),
        crate::player::ground::fresh_band(curve.fresh).into(),
    );
    if let Some(i) = issue {
        // How the worry landed (the player has just heard
        // the doorsteps, so this tells nothing hidden).
        res.args.insert("issue".into(), i.clone());
        res.args.insert("fit".into(), fit_band(content, fit).into());
    }
    res.args.insert(
        "place".into(),
        state.world.neighborhoods[hood.index()].name.clone(),
    );
    res
}
