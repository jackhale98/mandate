//! The people the player knows, as rows (DEC-113; `POCKET_REWORK.md` §3.1,
//! §3.3): one query, [`people_view`], lists every person the player has
//! heard of, in sections and in order, with why each matters now. It
//! replaces one `QueryDto::Character` a card.
//!
//! **The engine orders; the UI only lays out** (rule 4). A row carries its
//! section (`group`) and its place in it (`rank`); the view carries each
//! section's count. The UI sorts and counts nothing of its own.
//!
//! **Who is listed (DEC-091).** Everyone at `heard` or more in the ledger,
//! and everyone named by what is aimed at the player, which is never gated:
//! an open ask's asker and people, a pending decision's people, the cast
//! person a known arc's open beat is done with, the rival. Nobody else: a
//! member who has a vote on the player's bill is given that reason only if
//! the player has heard of them.
//!
//! **Sections (DEC-098).** `now` (any reason holds), `office` (a post in
//! one of the player's focus jurisdictions, read from the constitution),
//! `gone` (out of office after holding a post and not standing, or living
//! outside the focus), `rest`.
//!
//! Views never write.

use crate::knowledge::{Gate, KnowledgeLevelDto, SeenView, last_seen, usually_seen};
use crate::notebook::Notes;
use crate::places::RouteView;
use crate::pocket::{NowKind, NowReasonView, OrderKey, TextView, order_key, route_to};
use mandate_sim::Engine;
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::player::knowledge::{Level, Subject};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use ts_rs::TS;

/// A section of the people list, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PersonGroup {
    /// They matter now (`PersonRowView.now` says why).
    Now,
    /// In office in one of the player's focus jurisdictions.
    Office,
    /// Everyone else the player knows.
    Rest,
    /// Moved on or retired.
    Gone,
}

/// How someone stands with the player, as one chip: a word (`band`) with a
/// shape, never colour alone (DEC-051).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StanceView {
    /// Their opinion of the player: `hostile` | `cool` | `neutral` |
    /// `warm` | `friendly`.
    pub band: String,
    /// `down` (hostile, cool) | `level` (neutral) | `up` (warm, friendly).
    pub shape: String,
}

/// One person as a row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PersonRowView {
    pub id: u32,
    pub name: String,
    /// Fluent key of their role (`role-<word>`, `role-candidate`, a
    /// trade's name key, `role-private`).
    pub role_key: String,
    pub ward_number: Option<u32>,
    pub party: Option<u32>,
    /// What the player knows of them (for tests; never drawn).
    pub level: KnowledgeLevelDto,
    pub stance: StanceView,
    /// As `CharacterView` gives them: how they see the player is never gated.
    pub opinion_band: String,
    pub trust_band: String,
    /// How well they know the player's face, in words (`familiar-*`,
    /// DEC-114 §5); none for a stranger. Never a count.
    pub familiar_key: Option<String>,
    pub owes_you: u32,
    pub you_owe: u32,
    pub is_rival: bool,
    /// Which section the row is in.
    pub group: PersonGroup,
    /// Its order inside the section (0 first). The UI sorts by nothing else.
    pub rank: u32,
    /// Why they matter now, strongest first; empty unless `group` is `now`.
    pub now: Vec<NowReasonView>,
    /// They are where the player is, and can be spoken to.
    pub here: bool,
    pub can_talk: bool,
    /// Where "Go" walks: where their day has them now, if the player can
    /// go there (`action`: `talk`).
    pub go: Option<RouteView>,
    pub usually: Option<SeenView>,
    pub last_seen: Option<SeenView>,
    /// Knowledge in words, only where it changes something (§3.4).
    pub note: Option<TextView>,
    /// Learned of in play this turn and not yet seen in the notebook.
    pub new_this_turn: bool,
}

/// A section's count: how many rows are inside (never a share).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PeopleGroupView {
    pub group: PersonGroup,
    pub count: u32,
    /// How many rows the section shows before "N more" (the first `show`
    /// by `rank`).
    pub show: u32,
}

/// Everyone the player has heard of (`QueryDto::People`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PeopleView {
    /// In section order, then by `rank`.
    pub rows: Vec<PersonRowView>,
    /// The sections that have rows, in display order.
    pub groups: Vec<PeopleGroupView>,
}

/// Rows shown in "matter now" before "N more", and in a fold once opened.
const SHOW_NOW: u32 = 8;
const SHOW_FOLD: u32 = 12;

fn stance(band: &str) -> StanceView {
    StanceView {
        band: band.into(),
        shape: match band {
            "hostile" | "cool" => "down",
            "warm" | "friendly" => "up",
            _ => "level",
        }
        .into(),
    }
}

/// The reasons that name someone whatever the ledger says: what is aimed
/// at the player is never gated (DEC-091).
const NAMING: [NowKind; 4] = [
    NowKind::Ask,
    NowKind::Decision,
    NowKind::Arc,
    NowKind::Rival,
];

/// The people list.
pub fn people_view(engine: &Engine) -> Option<PeopleView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let me = characters::player_character(st)?;
    let pol = &st.politics;
    let gate = Gate::of(engine);
    let notes = Notes::of(engine);
    let matters = &notes.matters;
    let focus = pl.focus();
    let home = pl.home();

    let mut ids: BTreeSet<CharacterId> = pl
        .knowledge
        .entries
        .iter()
        .filter_map(|(s, e)| match s {
            Subject::Person { person } if e.level >= Level::Heard => Some(*person),
            _ => None,
        })
        .collect();
    ids.extend(
        matters
            .people()
            .filter(|id| NAMING.iter().any(|k| matters.has(*id, *k))),
    );
    ids.remove(&me);

    // (order within its section, id for ties, row)
    let mut rows: Vec<(OrderKey, u32, PersonRowView)> = Vec::new();
    for id in ids {
        let Some(p) = characters::profile(pol, &st.world, id) else {
            continue;
        };
        let level = gate.person(id);
        let politician = pol
            .characters
            .get(id)
            .and_then(|c| c.politician)
            .and_then(|x| pol.get(x));
        let loc = characters::routines::whereabouts(st, content, id);
        let here = loc == Some(pl.location);
        // A vote names only someone the player has heard of.
        let now: Vec<NowReasonView> = matters
            .person(engine, id, here)
            .into_iter()
            .filter(|r| r.kind != NowKind::Vote || level >= Level::Heard)
            .collect();
        let in_office = politician
            .is_some_and(|x| x.role.post().is_some_and(|post| focus.contains(&post.at())));
        let retired = politician.is_some_and(|x| {
            !x.in_office() && x.last_post.is_some() && !characters::is_candidate(pol, x.id)
        });
        let away = characters::municipality_of(pol, &st.world, id).is_some_and(|m| m != home.0);
        let group = if !now.is_empty() {
            PersonGroup::Now
        } else if in_office {
            PersonGroup::Office
        } else if retired || away {
            PersonGroup::Gone
        } else {
            PersonGroup::Rest
        };
        let seen = last_seen(engine, id);
        // Outside "matter now": most recently seen first.
        let recent = u32::MAX - seen.as_ref().map_or(0, |s| s.on.day_number);
        let key = if group == PersonGroup::Now {
            order_key(&now, here)
        } else {
            (recent, false, NowKind::Here)
        };
        let band = characters::opinion(pol, id, me) as f64;
        let band = crate::characters::opinion_band(band);
        let note = notes.person(id, &now);
        rows.push((
            key,
            id.0,
            PersonRowView {
                id: id.0,
                name: p.full_name(),
                role_key: crate::characters::label_role_key(engine, id),
                ward_number: politician
                    .and_then(|x| x.role.constituency())
                    .and_then(|w| st.world.constituencies.get(w.index()))
                    .map(|w| w.number),
                party: politician.and_then(|x| x.party).map(|x| x.0),
                level: level.into(),
                stance: stance(&band),
                opinion_band: band,
                familiar_key: crate::characters::familiar_key(engine, id),
                trust_band: crate::characters::trust_band(
                    characters::trust(pol, content, id, me) as f64
                ),
                owes_you: characters::favours_owed(pol, id, me),
                you_owe: characters::favours_owed(pol, me, id),
                is_rival: pol
                    .characters
                    .towns
                    .get(home.index())
                    .is_some_and(|t| t.rival == Some(id)),
                group,
                rank: 0,
                now,
                here,
                can_talk: here,
                go: loc.and_then(|l| route_to(engine, "talk", l)),
                usually: usually_seen(engine, id),
                last_seen: seen,
                note,
                new_this_turn: pl
                    .knowledge
                    .entry(&Subject::Person { person: id })
                    .is_some_and(|e| crate::knowledge::is_new(engine, e)),
            },
        ));
    }
    rows.sort_by_key(|a| (a.2.group, a.0, a.1));
    let mut groups: Vec<PeopleGroupView> = Vec::new();
    let mut out = Vec::with_capacity(rows.len());
    for (_, _, mut row) in rows {
        match groups.last_mut().filter(|g| g.group == row.group) {
            Some(g) => {
                row.rank = g.count;
                g.count += 1;
            }
            None => groups.push(PeopleGroupView {
                group: row.group,
                count: 1,
                show: if row.group == PersonGroup::Now {
                    SHOW_NOW
                } else {
                    SHOW_FOLD
                },
            }),
        }
        out.push(row);
    }
    Some(PeopleView { rows: out, groups })
}
