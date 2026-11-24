//! Reply composition (§3.5), pure: an opener by the relationship, sometimes
//! a memory quote, the substance by the facts (or the topic's fallback for
//! the speaker's role), sometimes a closer in the speaker's voice.
//!
//! **Repetition guard.** Lines in the speaker's used-lines memory are left
//! out while any other candidate remains (if all were used, the one used
//! longest ago is said). **The draw** is keyed: `rng::keyed(seed,
//! Stream::Dialogue, [TAG_TALK, speaker, hash(topic), week, part, n])`,
//! where `n` counts this (speaker, topic) pair's earlier conversations this
//! week, so asking again gives another line. No sequential stream advances
//! and views can call this freely.
//!
//! **The opener is the greeting** (M6 note 19): the person speaks as the
//! conversation opens, before any topic is chosen. Its draw is keyed on
//! [`GREETING`] in place of the topic, with `n` counting every earlier
//! conversation with this person this week, so the talk view's greeting
//! ([`opener`] with no topic) and a reply's opener follow one rule. The
//! first reply of the day says none: the greeting took its place.

use super::context::{Args, Ctx};
use super::state::UsedLine;
use mandate_content::conversation::{TalkLineDef, TalkPart, TopicDef, VoiceDef};
use mandate_core::Stream;

/// Keyed-RNG tag of conversation draws (DEC-041): never reused.
pub const TAG_TALK: u64 = 0x7A1C;

/// What the opener's draw is keyed on in place of a topic: the greeting
/// comes before any topic is chosen.
pub const GREETING: &str = "greeting";

/// How a reply opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opening {
    /// The conversation opened with the greeting (the talk view's): the reply
    /// says no opener of its own.
    Greeted,
    /// An opener, drawn as the greeting is, `n` being the earlier
    /// conversations with this person this week (any topic).
    Opener { n: u32 },
}

/// One chosen part of a reply, with the arguments its conditions gave.
#[derive(Clone, Debug)]
pub struct Chosen<'c> {
    pub part: TalkPart,
    pub line: &'c TalkLineDef,
    pub args: Args,
}

/// A composed reply, parts in order.
#[derive(Clone, Debug)]
pub struct Composed<'c> {
    pub voice: Option<&'c VoiceDef>,
    pub parts: Vec<Chosen<'c>>,
    /// The substance came from the topic's fallback lines.
    pub fell_back: bool,
}

/// The speaker's voice: the highest-priority voice whose conditions all
/// hold (ties: load order). A voice whose conditions name a voice (refused
/// by validation) is skipped, so the `voice` condition cannot recurse.
pub fn voice<'c>(ctx: &Ctx<'c>) -> Option<&'c VoiceDef> {
    let mut best: Option<&VoiceDef> = None;
    for v in &ctx.content.talk_voices {
        if best.is_some_and(|b| b.priority >= v.priority) || v.when.iter().any(names_voice) {
            continue;
        }
        if ctx.all_hold(&v.when, &mut Args::new()) {
            best = Some(v);
        }
    }
    best
}

fn names_voice(c: &mandate_content::Condition) -> bool {
    use mandate_content::Condition as U;
    match c {
        U::Voice { .. } => true,
        U::Any { of } => of.iter().any(names_voice),
        _ => false,
    }
}

fn holding<'c>(
    ctx: &Ctx<'c>,
    lines: impl Iterator<Item = &'c TalkLineDef>,
) -> Vec<(&'c TalkLineDef, Args)> {
    lines
        .filter_map(|l| {
            let mut a = Args::new();
            ctx.all_hold(&l.when, &mut a).then_some((l, a))
        })
        .collect()
}

/// The candidates for each part (§3.5 step 2).
pub fn candidates<'c>(
    ctx: &Ctx<'c>,
    topic: &TopicDef,
    part: TalkPart,
    voice: Option<&VoiceDef>,
) -> (Vec<(&'c TalkLineDef, Args)>, bool) {
    let lines = &ctx.content.talk_lines;
    let of = |p: TalkPart| lines.iter().filter(move |l| l.part == p);
    match part {
        TalkPart::Opener => (opener_candidates(ctx, Some(topic)), false),
        TalkPart::Quote => (holding(ctx, of(part)), false),
        TalkPart::Substance => {
            let mine = of(part).filter(|l| l.topic.as_deref() == Some(topic.id.as_str()));
            let real = holding(ctx, mine.clone().filter(|l| !l.fallback));
            // Asked about something (an `inform` topic with a subject), the
            // reply answers it: lines that read the subject first; if none
            // holds, the topic's safe answers, never a line about the topic
            // in general ("what worries people?" asked of one neighbourhood
            // is not answered with the town's worries). Only which line is
            // shown changes: the odds are fixed before the reply.
            if ctx.about.is_some()
                && topic.mechanic == mandate_content::conversation::TalkMechanic::Inform
            {
                let on: Vec<_> = real
                    .iter()
                    .filter(|(l, _)| mandate_content::conversation::reads_about(&l.when))
                    .cloned()
                    .collect();
                if !on.is_empty() {
                    return (on, false);
                }
                let safe = holding(ctx, mine.clone().filter(|l| l.fallback));
                if !safe.is_empty() {
                    return (safe, true);
                }
            }
            if !real.is_empty() {
                return (real, false);
            }
            (holding(ctx, mine.filter(|l| l.fallback)), true)
        }
        TalkPart::Closer => {
            let Some(v) = voice else {
                return (Vec::new(), false);
            };
            (
                holding(
                    ctx,
                    of(part).filter(|l| l.voice.as_deref() == Some(v.id.as_str())),
                ),
                false,
            )
        }
    }
}

/// The openers that hold: a topic's own first (none before a topic is
/// chosen), else those for any topic.
fn opener_candidates<'c>(ctx: &Ctx<'c>, topic: Option<&TopicDef>) -> Vec<(&'c TalkLineDef, Args)> {
    let of = || {
        ctx.content
            .talk_lines
            .iter()
            .filter(|l| l.part == TalkPart::Opener)
    };
    if let Some(t) = topic {
        let own = holding(
            ctx,
            of().filter(|l| l.topic.as_deref() == Some(t.id.as_str())),
        );
        if !own.is_empty() {
            return own;
        }
    }
    holding(ctx, of().filter(|l| l.topic.is_none()))
}

/// The opener (the greeting, with no topic): the candidates that hold, the
/// repetition guard, then the keyed draw on [`GREETING`]. `n`: the earlier
/// conversations with this person this week.
pub fn opener<'c>(
    ctx: &Ctx<'c>,
    topic: Option<&TopicDef>,
    used: &[UsedLine],
    week: u32,
    n: u32,
) -> Option<Chosen<'c>> {
    let cands = opener_candidates(ctx, topic);
    let mut rng = draw_rng(ctx, GREETING, week, TalkPart::Opener, n);
    // The same draws as every part: the chance, then the line.
    let _ = rng.next_f64();
    if cands.is_empty() {
        return None;
    }
    let cands = guard(cands, used);
    let weights: Vec<f64> = cands.iter().map(|(l, _)| l.weight).collect();
    let i = rng.weighted_index(&weights)?;
    let (line, args) = cands.into_iter().nth(i)?;
    Some(Chosen {
        part: TalkPart::Opener,
        line,
        args,
    })
}

/// Drop lines the speaker has used while others remain; if every one was
/// used, keep the one used longest ago.
pub fn guard<'c>(
    cands: Vec<(&'c TalkLineDef, Args)>,
    used: &[UsedLine],
) -> Vec<(&'c TalkLineDef, Args)> {
    let pos = |id: &str| used.iter().position(|u| u.line == id);
    if cands.iter().any(|(l, _)| pos(&l.id).is_none()) {
        return cands
            .into_iter()
            .filter(|(l, _)| pos(&l.id).is_none())
            .collect();
    }
    let oldest = cands.iter().filter_map(|(l, _)| pos(&l.id)).min();
    cands
        .into_iter()
        .filter(|(l, _)| pos(&l.id) == oldest)
        .take(1)
        .collect()
}

/// The keyed draw for one part.
pub fn draw_rng(
    ctx: &Ctx<'_>,
    topic: &str,
    week: u32,
    part: TalkPart,
    n: u32,
) -> mandate_core::SimRng {
    mandate_core::rng::keyed(
        ctx.st.meta.seed,
        Stream::Dialogue,
        &[
            TAG_TALK,
            ctx.speaker.0 as u64,
            mandate_core::hash::derive_u64(topic, &[]),
            week as u64,
            part.tag(),
            n as u64,
        ],
    )
}

/// Compose a reply (§3.5 steps 1–5) with an opener (keyed on `n` too).
/// `used` is the speaker's used-lines memory, `n` this pair's earlier
/// conversations this week.
pub fn compose<'c>(
    ctx: &Ctx<'c>,
    topic: &TopicDef,
    used: &[UsedLine],
    week: u32,
    n: u32,
) -> Composed<'c> {
    compose_reply(ctx, topic, used, week, n, Opening::Opener { n })
}

/// Compose a reply, opening as `opening` says.
pub fn compose_reply<'c>(
    ctx: &Ctx<'c>,
    topic: &TopicDef,
    used: &[UsedLine],
    week: u32,
    n: u32,
    opening: Opening,
) -> Composed<'c> {
    let v = voice(ctx);
    let mut parts = Vec::new();
    let mut fell_back = false;
    for part in TalkPart::ALL {
        if part == TalkPart::Opener {
            if let Opening::Opener { n } = opening
                && let Some(c) = opener(ctx, Some(topic), used, week, n)
            {
                parts.push(c);
            }
            continue;
        }
        let (cands, fb) = candidates(ctx, topic, part, v);
        let mut rng = draw_rng(ctx, &topic.id, week, part, n);
        // A constant number of draws per part, whatever holds.
        let chance = rng.next_f64();
        let share = match part {
            TalkPart::Quote => ctx.content.talk.quote_share,
            TalkPart::Closer => v.map(|v| v.closer_share).unwrap_or(0.0),
            TalkPart::Opener | TalkPart::Substance => 1.0,
        };
        if cands.is_empty() || chance >= share {
            continue;
        }
        let cands = guard(cands, used);
        let weights: Vec<f64> = cands.iter().map(|(l, _)| l.weight).collect();
        let Some(i) = rng.weighted_index(&weights) else {
            continue;
        };
        let Some((line, args)) = cands.into_iter().nth(i) else {
            continue;
        };
        if part == TalkPart::Substance {
            fell_back = fb;
        }
        parts.push(Chosen { part, line, args });
    }
    Composed {
        voice: v,
        parts,
        fell_back,
    }
}
