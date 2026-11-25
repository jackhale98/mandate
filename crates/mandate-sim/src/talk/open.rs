//! The open question, "What's on your mind?" (M6 note 26): the person says
//! what they most want to say, chosen by the engine from their own state.
//!
//! What they may bring up (`talk_params.open.order`, first that they have
//! and have not already said to the player this week):
//! - **an ask:** something they have asked of the player and are still
//!   waiting on (an open ask whose asker they are; the asks' own state);
//! - **their main worry:** what is felt most where they live (the reply is
//!   about their own neighbourhood);
//! - **gossip:** about someone they know plainly (a colleague first, else
//!   someone in office), the same person all week.
//!
//! The reply is then composed as for any topic, about what was chosen
//! (`AboutRef`), so the same lines, discretion and repetition guard apply:
//! how much is told still follows their opinion of and trust in the player.
//! With nothing on their mind, the topic's safe answer is said (a miss).
//!
//! Read-only. The gossip pick is a keyed draw (DEC-041) on the person and
//! the week: no stream advances.

use super::AboutRef;
use crate::characters::{self, CharacterId, knows};
use crate::player::knowledge::{Level, Subject};
use crate::state::State;
use crate::world::{MunicipalityId, NeighborhoodId};
use mandate_content::ContentBundle;
use mandate_content::conversation::MindKind;
use mandate_core::Stream;

/// Keyed-RNG tag of the gossip pick (DEC-041): never reused.
pub const TAG_OPEN: u64 = 0x7A1D;

/// One thing on someone's mind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mind {
    pub kind: MindKind,
    /// What the reply is about (`None` for an ask: it is about the player).
    pub about: Option<AboutRef>,
}

/// Whether the person has asked something of the player and is waiting on
/// it: an open ask (offered or accepted, not a campaign promise) of theirs.
pub fn has_ask(state: &State, speaker: CharacterId) -> bool {
    state
        .asks
        .asks
        .iter()
        .any(|a| a.open() && !a.promise && a.asker_person() == Some(speaker))
}

/// Their home neighbourhood, when it is in the town of the conversation.
fn home_in(state: &State, speaker: CharacterId, town: MunicipalityId) -> Option<NeighborhoodId> {
    let home = characters::profile(&state.politics, &state.world, speaker)
        .map(|p| NeighborhoodId(p.home))?;
    let h = state.world.neighborhoods.get(home.index())?;
    (h.municipality == town).then_some(home)
}

/// Whom they would gossip about this week: someone in town they know
/// plainly (never themselves or the player), a colleague where they have
/// one, else someone in office; one of them, by a keyed draw.
pub fn gossip_about(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    me: CharacterId,
    town: MunicipalityId,
) -> Option<CharacterId> {
    let pol = &state.politics;
    let place = |c: CharacterId| {
        pol.characters
            .get(c)
            .and_then(|x| x.named.as_deref()?.place)
    };
    let mut colleagues = Vec::new();
    let mut others = Vec::new();
    for (i, _) in pol.characters.people.iter().enumerate() {
        let c = CharacterId(i as u32);
        if c == speaker || c == me {
            continue;
        }
        if characters::municipality_of(pol, &state.world, c) != Some(town.0) {
            continue;
        }
        let known = knows::speaker_knows(state, content, speaker, &Subject::Person { person: c });
        if known < Level::Known {
            continue;
        }
        if place(speaker).is_some() && place(speaker) == place(c) {
            colleagues.push(c);
        } else {
            others.push(c);
        }
    }
    let pool = if colleagues.is_empty() {
        others
    } else {
        colleagues
    };
    if pool.is_empty() {
        return None;
    }
    let week = super::week_of(state.calendar.date);
    let mut rng = mandate_core::rng::keyed(
        state.meta.seed,
        Stream::Dialogue,
        &[TAG_OPEN, speaker.0 as u64, week as u64],
    );
    let i = (rng.next_f64() * pool.len() as f64) as usize;
    pool.get(i.min(pool.len() - 1)).copied()
}

/// Everything on the person's mind, in the content's order.
pub fn minds(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    me: CharacterId,
    town: MunicipalityId,
) -> Vec<Mind> {
    let mut out = Vec::new();
    for kind in &content.talk.open.order {
        let about = match kind {
            MindKind::Ask => has_ask(state, speaker).then_some(None),
            MindKind::Worry => {
                home_in(state, speaker, town).map(|hood| Some(AboutRef::Hood { hood }))
            }
            MindKind::Gossip => gossip_about(state, content, speaker, me, town)
                .map(|person| Some(AboutRef::Person { person })),
        };
        if let Some(about) = about {
            out.push(Mind { kind: *kind, about });
        }
    }
    out
}

/// What the person says when asked now: the first thing on their mind they
/// have not already said to the player this week (asked again once all are
/// said: the first again). `None`: nothing on their mind.
pub fn mind(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    me: CharacterId,
    town: MunicipalityId,
    topic: &str,
) -> Option<Mind> {
    let all = minds(state, content, speaker, me, town);
    let week = super::week_of(state.calendar.date);
    let said: Vec<&Option<AboutRef>> = state
        .player
        .as_ref()
        .map(|pl| {
            pl.talk
                .this_week(speaker, week)
                .filter(|e| e.topic == topic)
                .map(|e| &e.about)
                .collect()
        })
        .unwrap_or_default();
    all.iter()
        .find(|m| !said.contains(&&m.about))
        .or(all.first())
        .cloned()
}
