//! What a person could be expected to know (M6 note 26): the order of the
//! talk panel.
//!
//! **Public facts only.** For each topic and each subject it can be about,
//! the engine works out how well this person would know it, judged from
//! what anyone can see of them: their occupation, where they live and work,
//! the post they hold, the bills they sponsor, the issue their trade speaks
//! for. The panel lists the likely subjects first and puts the rest behind
//! "Something else…". It never reads what the person secretly knows (who
//! the broker is), nor what they are willing to say (their opinion of the
//! player, their trust, their traits): the menu must not give the answer
//! away. Every subject stays askable.
//!
//! **No nonsense pairings** ([`nonsense`]): a topic is not offered about its
//! own speaker, about the place both stand in to someone who sees it only
//! from outside, or on a bill to the person whose bill it is. By rule, not
//! by list.
//!
//! **Who would know** ([`who_knows`]): the person the speaker knows who is
//! expected to know a subject best, for the lead a miss leaves.
//!
//! Read-only; nothing is stored and no random number is drawn.

use super::AboutRef;
use super::context::Ctx;
use crate::characters::{self, CharacterId, knows};
use crate::places::Location;
use crate::player::knowledge::{Level, PolicyRef, Subject};
use crate::world::NeighborhoodId;
use mandate_content::conversation::{TalkMechanic, TopicDef};

/// The post a person holds, if any.
fn post_of(ctx: &Ctx<'_>, c: CharacterId) -> Option<crate::politics::Post> {
    let pol = &ctx.st.politics;
    pol.characters
        .get(c)?
        .politician
        .and_then(|p| pol.get(p))
        .and_then(|p| p.role.post())
}

/// The issue a person's trade speaks for (their occupation's `expertise`):
/// public, unlike a politician's private pet issue.
fn public_issue<'a>(ctx: &Ctx<'a>, c: CharacterId) -> Option<&'a str> {
    let named = ctx.st.politics.characters.get(c)?.named.as_deref()?;
    ctx.content
        .occupation(&named.occupation)?
        .expertise
        .as_deref()
}

/// Whether a policy template is chiefly about this issue: the issue is the
/// strongest axis of its signature.
fn chiefly_about(ctx: &Ctx<'_>, template: &str, issue: &str) -> bool {
    let Some(t) = ctx.content.policies.get(template) else {
        return false;
    };
    let top = t.signature.iter().map(|(k, v)| (k.as_str(), v.abs())).fold(
        None::<(&str, f64)>,
        |best, (k, v)| match best {
            Some((_, b)) if b >= v => best,
            _ => Some((k, v)),
        },
    );
    top.is_some_and(|(k, _)| k == issue)
}

/// How well public facts say the speaker would know a place: where they
/// work (from inside), a kind of place their trade is found at, somewhere
/// in their own neighbourhood or ward.
fn expected_place(ctx: &Ctx<'_>, place: crate::world::PlaceId) -> Level {
    let st = ctx.st;
    let Some(p) = st.world.places.get(place.index()) else {
        return Level::Unknown;
    };
    if ctx.workplace(ctx.speaker).is_some_and(|w| w.id == p.id) {
        return Level::Detailed;
    }
    let trade = st
        .politics
        .characters
        .get(ctx.speaker)
        .and_then(|c| c.named.as_deref())
        .and_then(|n| ctx.content.occupation(&n.occupation));
    if trade.is_some_and(|o| o.places.iter().any(|k| k == p.kind.key())) {
        return Level::Known;
    }
    let home = characters::profile(&st.politics, &st.world, ctx.speaker).map(|x| x.home);
    let ward = post_of(ctx, ctx.speaker)
        .filter(|post| post.constituency().is_some())
        .map(|post| post.hoods(&st.world))
        .unwrap_or_default();
    if home == Some(p.neighborhood.0) || ward.contains(&p.neighborhood.index()) {
        return Level::Heard;
    }
    Level::Unknown
}

/// How well public facts say the speaker would know the support for a
/// policy: a politician counts heads on what is before the body (the
/// content rules), a sponsor knows their own bill, and anyone knows the
/// policies their trade is chiefly about. An idea not before the body is
/// expected only of someone whose trade it is.
fn expected_policy(ctx: &Ctx<'_>, about: &AboutRef) -> Level {
    let st = ctx.st;
    let issue = public_issue(ctx, ctx.speaker);
    match about {
        AboutRef::Bill { policy } => {
            let Some(b) = st.policy.instances.get(policy.index()) else {
                return Level::Unknown;
            };
            let me = st
                .politics
                .characters
                .get(ctx.speaker)
                .and_then(|c| c.politician);
            if b.sponsor.is_some() && b.sponsor == me {
                return Level::Detailed;
            }
            let rule = knows::expected_knows(
                st,
                ctx.content,
                ctx.speaker,
                &Subject::PolicySupport {
                    town: ctx.town,
                    policy: PolicyRef::Template {
                        template: b.template.clone(),
                    },
                },
            );
            if issue.is_some_and(|i| chiefly_about(ctx, &b.template, i)) {
                rule.max(Level::Heard)
            } else {
                rule
            }
        }
        AboutRef::Template { template } => {
            if issue.is_some_and(|i| chiefly_about(ctx, template, i)) {
                Level::Heard
            } else {
                Level::Unknown
            }
        }
        _ => Level::Unknown,
    }
}

/// How well public facts say the speaker would know what a topic asks
/// about this subject ([`Level::Unknown`]: no reason to expect it of them).
pub fn expectation(ctx: &Ctx<'_>, topic: &TopicDef, about: &AboutRef) -> Level {
    let st = ctx.st;
    let rule = |s: Subject| knows::expected_knows(st, ctx.content, ctx.speaker, &s);
    match about {
        AboutRef::Hood { hood } => rule(Subject::HoodConcerns { hood: *hood }),
        AboutRef::Jurisdiction { .. } => rule(Subject::TownConcerns { town: ctx.town }),
        AboutRef::Person { person } => rule(Subject::Person { person: *person }),
        AboutRef::Rival => {
            let plans = rule(Subject::RivalPlans { town: ctx.town });
            let person = ctx
                .rival()
                .map(|r| rule(Subject::Person { person: r }))
                .unwrap_or_default();
            plans.max(person)
        }
        AboutRef::Place { place } => expected_place(ctx, *place),
        AboutRef::Bill { policy } => {
            // A colleague who votes on it is the person to pitch it to, or
            // to ask for backing.
            let votes = topic.mechanic != TalkMechanic::Inform
                && post_of(ctx, ctx.speaker).is_some_and(|post| {
                    st.policy.instances.get(policy.index()).is_some_and(|b| {
                        post.at().municipality().map(|m| m.0) == Some(b.jurisdiction)
                    })
                });
            let known = expected_policy(ctx, about);
            if votes {
                known.max(Level::Known)
            } else {
                known
            }
        }
        AboutRef::Template { .. } => expected_policy(ctx, about),
        AboutRef::Proposal { .. } => Level::Unknown,
    }
}

/// Whether a subject is listed among the likely ones for this speaker
/// (`talk_params.likely_from`).
pub fn likely(ctx: &Ctx<'_>, topic: &TopicDef, about: &AboutRef) -> bool {
    let from = ctx.content.talk.likely_from.max(Level::Heard);
    expectation(ctx, topic, about) >= from
}

/// A pairing of topic, subject and speaker that makes no sense, whoever
/// they are:
/// - the subject is the speaker (someone asked about themselves in the
///   third person; the rival asked about "your rival");
/// - the subject is the place the conversation is in, asked of someone who
///   does not work there (what is said of a place from outside is said
///   elsewhere);
/// - a bill pitched to, or support or a favour sought on it from, the
///   person whose bill it is.
pub fn nonsense(ctx: &Ctx<'_>, topic: &TopicDef, about: Option<&AboutRef>) -> bool {
    let Some(a) = about else {
        return false;
    };
    // Read from the subject itself, not the conversation's bound `about`
    // (the panel asks before any subject is bound).
    let subject_person = match a {
        AboutRef::Person { person } => Some(*person),
        AboutRef::Rival => ctx.rival(),
        _ => None,
    };
    if subject_person == Some(ctx.speaker) {
        return true;
    }
    match a {
        AboutRef::Place { place } => {
            ctx.here == Location::Place { place: *place }
                && ctx.workplace(ctx.speaker).map(|w| w.id) != Some(*place)
        }
        AboutRef::Bill { policy } if topic.mechanic != TalkMechanic::Inform => {
            let me = ctx
                .st
                .politics
                .characters
                .get(ctx.speaker)
                .and_then(|c| c.politician);
            me.is_some()
                && ctx
                    .st
                    .policy
                    .instances
                    .get(policy.index())
                    .is_some_and(|b| b.sponsor == me)
        }
        _ => false,
    }
}

/// The person the speaker knows (plainly: a colleague, someone in office)
/// who would know a subject at least `min`, judged from public facts: the
/// one expected to know it best (ties: the lowest id). Never the speaker,
/// the player, or someone expected to know no more than the speaker.
pub fn who_knows(ctx: &Ctx<'_>, subject: &Subject, min: Level) -> Option<CharacterId> {
    let st = ctx.st;
    let own = knows::expected_knows(st, ctx.content, ctx.speaker, subject);
    let floor = min.max(Level::Heard);
    let mut best: Option<(Level, CharacterId)> = None;
    for (i, _) in st.politics.characters.people.iter().enumerate() {
        let c = CharacterId(i as u32);
        if c == ctx.speaker || c == ctx.me {
            continue;
        }
        if characters::municipality_of(&st.politics, &st.world, c) != Some(ctx.town.0) {
            continue;
        }
        let l = knows::expected_knows(st, ctx.content, c, subject);
        if l < floor || l <= own || best.is_some_and(|(b, _)| b >= l) {
            continue;
        }
        let known =
            knows::speaker_knows(st, ctx.content, ctx.speaker, &Subject::Person { person: c });
        if known >= Level::Known {
            best = Some((l, c));
        }
    }
    best.map(|(_, c)| c)
}

/// The neighbourhood a person lives in.
pub fn home_of(ctx: &Ctx<'_>, c: CharacterId) -> Option<NeighborhoodId> {
    characters::profile(&ctx.st.politics, &ctx.st.world, c).map(|p| NeighborhoodId(p.home))
}
