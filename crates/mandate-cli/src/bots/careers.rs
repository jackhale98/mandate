//! Whole-career bot runs (`bots --careers`).

use super::*;

/// A career's outcome over several years with a strategy (DEC-076 style
/// distribution for the corrupt strategy): elections won and lost, scandals,
/// investigations, convictions, prison, and a career ended.
pub fn career(
    content: &Arc<ContentBundle>,
    seed: u64,
    strategy: &str,
    years: u32,
) -> Option<serde_json::Value> {
    let mut e = new_game(content, seed)?;
    let me = e.state().player.as_ref()?.politician;
    let start = e.state().calendar.date;
    let end = start.add_days(years * 365);
    let start_turn = e.state().calendar.turn;
    let mut imprisoned = false;
    while e.state().calendar.date < end {
        play_turn_from(&mut e, content, strategy, start_turn);
        if e.state()
            .player
            .as_ref()
            .is_some_and(|p| matches!(p.status, mandate_sim::player::RunStatus::Imprisoned { .. }))
        {
            imprisoned = true;
        }
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
    }
    let st = e.state();
    let (mut won, mut lost) = (0, 0);
    for r in st
        .politics
        .elections
        .results
        .iter()
        .filter(|r| r.date > start)
    {
        if r.candidates.iter().any(|c| c.politician == me) {
            if r.winner == me {
                won += 1;
            } else {
                lost += 1;
            }
        }
    }
    let scandals = st.media.scandals.iter().filter(|s| s.subject == me).count();
    let cases: Vec<&mandate_sim::justice::Case> = st
        .justice
        .cases
        .iter()
        .filter(|c| c.subject == me)
        .collect();
    let convicted = cases
        .iter()
        .filter(|c| c.stage == mandate_sim::justice::Stage::Convicted)
        .count();
    let prison = cases
        .iter()
        .filter(|c| c.sentence.is_some_and(|s| s.prison_years() > 0))
        .count();
    let ended = st
        .player
        .as_ref()
        .is_some_and(|p| matches!(p.status, mandate_sim::player::RunStatus::Ended { .. }));
    let hidden = st
        .narrative
        .hidden
        .iter()
        .filter(|h| h.subject == Some(me))
        .count();
    let mut kinds: BTreeMap<String, u32> = BTreeMap::new();
    for h in st.narrative.hidden.iter().filter(|h| h.subject == Some(me)) {
        *kinds.entry(h.kind.clone()).or_default() += 1;
    }
    let npc_scandals = st.media.scandals.iter().filter(|s| s.subject != me).count();
    let npc_cases = st.justice.cases.iter().filter(|c| c.subject != me).count();
    Some(json!({
        "seed": seed, "won": won, "lost": lost, "hidden_acts": hidden, "acts": kinds, "scandals": scandals,
        "cases": cases.len(), "convicted": convicted, "prison": prison > 0 || imprisoned, "career_ended": ended,
        "in_office": st.politics.get(me).is_some_and(|p| p.in_office()),
        "money": st.player.as_ref().map(|p| p.money.units()).unwrap_or(0),
        "npc_scandals": npc_scandals, "npc_cases": npc_cases,
        "asks": st.asks.tally, "ask_templates": ask_templates(st),
    }))
}

/// Per ask or promise template still in the history: offered (or made),
/// kept, broken (a lab view; the whole-career counts are the tally).
pub(super) fn ask_templates(st: &mandate_sim::State) -> BTreeMap<String, [u32; 3]> {
    let mut out: BTreeMap<String, [u32; 3]> = BTreeMap::new();
    for a in &st.asks.asks {
        let e = out.entry(a.template.clone()).or_default();
        e[0] += 1;
        match a.status {
            mandate_sim::asks::AskStatus::Kept { .. } => e[1] += 1,
            mandate_sim::asks::AskStatus::Broken { .. } => e[2] += 1,
            _ => {}
        }
    }
    out
}

/// The career distribution of a strategy across seeds.
pub fn careers(
    content: &Arc<ContentBundle>,
    seeds: u64,
    strategy: &str,
    years: u32,
) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = (1..=seeds)
        .filter_map(|s| career(content, s, strategy, years))
        .collect();
    let n = rows.len().max(1) as f64;
    let count = |k: &str| {
        rows.iter()
            .filter(|r| r[k].as_bool().unwrap_or(false) || r[k].as_u64().unwrap_or(0) > 0)
            .count()
    };
    let sum = |k: &str| rows.iter().map(|r| r[k].as_u64().unwrap_or(0)).sum::<u64>();
    json!({
        "strategy": strategy, "seeds": seeds, "years": years,
        "elections_won": sum("won"), "elections_lost": sum("lost"),
        "careers_with_a_scandal": count("scandals") as f64 / n,
        "careers_investigated": count("cases") as f64 / n,
        "careers_convicted": count("convicted") as f64 / n,
        "careers_in_prison": count("prison") as f64 / n,
        "careers_ended": count("career_ended") as f64 / n,
        "careers_in_office_at_end": count("in_office") as f64 / n,
        "npc_scandals_per_career": sum("npc_scandals") as f64 / n,
        "npc_cases_per_career": sum("npc_cases") as f64 / n,
        "rows": rows,
    })
}
