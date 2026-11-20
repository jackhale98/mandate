//! `mandate-cli routines`: where the player's town's key people are in each
//! part of the day, turn by turn (DEC-031 daily routines), for checking a
//! routines pack. Each "day" is one turn's representative day; the game
//! advances a turn between days (the player idles).

use mandate_content::ContentBundle;
use mandate_sim::characters::routines::{self, DayPart, Now};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::places::Location;
use mandate_sim::world::Jurisdiction;
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::{Value, json};
use std::io::Write;
use std::sync::Arc;

/// Someone worth following and how they are labelled (a role word, an
/// occupation id, `broker`, `rival`).
fn key_people(st: &State, content: &ContentBundle, all: bool) -> Vec<(CharacterId, String)> {
    let Some(pl) = st.player.as_ref() else {
        return Vec::new();
    };
    let town = pl.home();
    let pol = &st.politics;
    let c = content.constitutions.get(&st.meta.constitution);
    let me = characters::player_character(st);
    let t = pol.characters.towns.get(town.index());
    let mut out: Vec<(CharacterId, String)> = Vec::new();
    let push = |id: CharacterId, label: String, out: &mut Vec<(CharacterId, String)>| {
        if Some(id) != me && !out.iter().any(|x| x.0 == id) {
            out.push((id, label));
        }
    };
    if let Some(b) = t.and_then(|t| t.broker) {
        push(b, "broker".into(), &mut out);
    }
    if let Some(r) = t.and_then(|t| t.rival) {
        push(r, "rival".into(), &mut out);
    }
    for pid in mandate_sim::politics::officials_at(pol, Jurisdiction::town(town)) {
        let Some(p) = pol.get(pid) else { continue };
        let word = c
            .and_then(|c| mandate_sim::politics::role_word(c, p))
            .unwrap_or("official");
        push(p.character, word.into(), &mut out);
    }
    let mut seen_occ: Vec<String> = Vec::new();
    for id in routines::townsfolk(st, town) {
        let Some(n) = pol.characters.get(id).and_then(|c| c.named.as_deref()) else {
            if all {
                push(id, "politician".into(), &mut out);
            }
            continue;
        };
        if all || !seen_occ.contains(&n.occupation) {
            seen_occ.push(n.occupation.clone());
            push(id, n.occupation.clone(), &mut out);
        }
    }
    out
}

fn where_label(st: &State, loc: Location) -> String {
    match loc {
        Location::Home => "home".into(),
        Location::Street { neighborhood } => format!(
            "streets of {}",
            st.world
                .neighborhoods
                .get(neighborhood.index())
                .map(|h| h.name.as_str())
                .unwrap_or("?")
        ),
        Location::Place { place } => st
            .world
            .places
            .get(place.index())
            .map(|p| {
                if p.name.is_empty() {
                    p.kind.key().to_string()
                } else {
                    format!("{} ({})", p.name, p.kind.key())
                }
            })
            .unwrap_or_else(|| "?".into()),
    }
}

fn events(now: &Now, st: &State) -> Vec<&'static str> {
    let Some(pl) = st.player.as_ref() else {
        return Vec::new();
    };
    let at = Jurisdiction::town(pl.home());
    let mut v = Vec::new();
    if now.session(st.meta.seed, at) {
        v.push("session");
    }
    if now.votes.contains(&at) {
        v.push("vote");
    }
    if now.campaigns.contains(&at) {
        v.push("campaign");
    }
    if routines::meeting_night(st.meta.seed, pl.home()) == now.weekday {
        v.push("meeting_night");
    }
    v
}

/// Play `days` turns (idle) and report, for each, where the key people are
/// in each part of the day. With `text`, one line per person and day.
pub fn run(
    content: &Arc<ContentBundle>,
    seed: u64,
    days: u32,
    size: WorldSize,
    all: bool,
    text: bool,
) -> Result<Value, String> {
    let mut e: Engine = generate(
        &WorldConfig {
            seed,
            size,
            scenario: None,
        },
        content.clone(),
    )
    .map_err(|e| e.to_string())?;
    let people = key_people(e.state(), content, all);
    let mut rows = Vec::new();
    for _ in 0..days {
        let st = e.state();
        let nows: Vec<Now> = DayPart::ALL
            .iter()
            .map(|p| Now::at(st, content, *p))
            .collect();
        let now = &nows[0];
        let ev = events(now, st);
        let mut who = Vec::new();
        for (id, label) in &people {
            let name = characters::profile(&st.politics, &st.world, *id)
                .map(|p| p.full_name())
                .unwrap_or_default();
            let mut at = serde_json::Map::new();
            for n in &nows {
                let l = routines::locate(st, content, n, *id)
                    .map(|l| where_label(st, l))
                    .unwrap_or_else(|| "nowhere".into());
                at.insert(n.part.key().into(), l.into());
            }
            if text {
                let d = now.day.calendar();
                let _ = writeln!(
                    std::io::stdout().lock(),
                    "{}-{:02}-{:02} {} [{}] {:<24} {:<20} | {}",
                    d.year,
                    d.month,
                    d.day,
                    now.weekday.key(),
                    ev.join(","),
                    label,
                    name,
                    DayPart::ALL
                        .iter()
                        .map(|p| at
                            .get(p.key())
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string())
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
            }
            who.push(json!({ "id": id.0, "who": label, "name": name, "at": at }));
        }
        rows.push(json!({
            "turn": st.calendar.turn,
            "date": st.calendar.date.0,
            "representative_day": now.day.0,
            "weekday": now.weekday.key(),
            "season": format!("{:?}", now.season).to_lowercase(),
            "events": ev,
            "people": who,
        }));
        e.apply(Command::EndTurn).map_err(|e| e.to_string())?;
    }
    Ok(json!({ "seed": seed, "days": rows }))
}
