//! Lobbying is a conversation (M6 note 29; DEC-093, DEC-091, DEC-073,
//! DEC-030). The one-tap "Lobby" is gone for the player: a member is talked
//! round on a bill, and what the player can argue comes from what they know
//! of them. Each argument is a choice in the conversation (S4 talk hooks,
//! owner `lobby`; id `choice.lobby.<bill>.<argument>`), with its chance and
//! its cost shown before it is chosen:
//!
//! - **The plain pitch** is always there (DEC-091's principle: knowledge
//!   opens other options, never the only way through): the lobbying odds,
//!   which carry how well the player knows the member.
//! - **Their ward:** once the player knows the worries of a neighbourhood
//!   in the member's ward, they can argue from what the ward thinks of the
//!   bill in place of what the member thinks of the player.
//! - **Their party's line,** when the member's party is on the player's
//!   side of it: it works on loyal members.
//! - **Your word,** when the player kept something the member asked of
//!   them: it works by trust.
//! - **The broker,** once the player knows who the broker is and the broker
//!   wants what the player wants: it works on those who go along with them.
//! - **A favour:** called in (certain; it is spent) or offered (certain; the
//!   player owes them).
//! - **Meet after hours:** the plain pitch in the late slot, with the
//!   late-night bonus, its health cost and a back-room deal's legal risk.
//! - **Bring others:** an ally of the player, or the whip of the player's
//!   party, can be asked to bring the members round who like them or follow
//!   the party; the player owes them for it.
//!
//! Two exchanges a member and bill a week; a member who has committed (a
//! declared vote, or the bill's sponsor) is settled and offers none.
//! Lobbying needs no walk: it can be had from the whip's list (DEC-051).
//!
//! [`pitch`] is the one function behind the chance and cost shown, the
//! command, and the NPCs' lobbying (DEC-030: `npc` lobbies with the plain
//! pitch through it, rolled by [`roll`]). The odds shown are the odds
//! rolled (DEC-073): the roll uses the choice's odds, fixed before the
//! conversation teaches or changes anything. Facts and Fluent keys only.

use super::*;
use crate::asks::{Asker, History};
use crate::characters::town::{self, broker_pull, compliance};
use crate::characters::{Bond, CharacterId};
use crate::spine::{Cause, PlayerAct};
use crate::talk::hooks::{self, Choice, ChoiceCost, Done, Hook, Reply};
use mandate_core::math::sigmoid;

/// The lobbying hook (registered in `talk::hooks::HOOKS`).
pub const HOOK: Hook = Hook {
    owner: "lobby",
    offer,
    chosen,
};

/// An argument the player can make to a member on a bill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Argument {
    Plain,
    Ward,
    Party,
    Word,
    Broker,
    CallIn,
    Owe,
    AfterHours,
    Rally,
}

impl Argument {
    /// Every argument, in the order the conversation lists them.
    pub const ALL: [Argument; 9] = [
        Argument::Plain,
        Argument::Ward,
        Argument::Party,
        Argument::Word,
        Argument::Broker,
        Argument::Rally,
        Argument::CallIn,
        Argument::Owe,
        Argument::AfterHours,
    ];

    /// The id's last part and the Fluent keys' (`lobby-choice-<word>`).
    pub fn word(self) -> &'static str {
        match self {
            Argument::Plain => "plain",
            Argument::Ward => "ward",
            Argument::Party => "party",
            Argument::Word => "word",
            Argument::Broker => "broker",
            Argument::CallIn => "call-in",
            Argument::Owe => "owe",
            Argument::AfterHours => "after-hours",
            Argument::Rally => "rally",
        }
    }

    pub fn parse(w: &str) -> Option<Argument> {
        Argument::ALL.into_iter().find(|a| a.word() == w)
    }

    /// Whether it is rolled (else it cannot fail).
    pub fn rolled(self) -> bool {
        !matches!(self, Argument::CallIn | Argument::Owe)
    }
}

/// `choice.lobby.<bill>.<argument>`: the `Talk` command's `topic`.
pub fn choice_id(policy: u32, a: Argument) -> String {
    hooks::choice_id(HOOK.owner, &format!("{policy}.{}", a.word()))
}

/// The bill and the argument a choice id names.
pub fn parse_id(id: &str) -> Option<(u32, Argument)> {
    let local = id
        .strip_prefix(hooks::PREFIX)?
        .strip_prefix(HOOK.owner)?
        .strip_prefix('.')?;
    let (policy, word) = local.split_once('.')?;
    Some((policy.parse().ok()?, Argument::parse(word)?))
}

/// The prefix of every choice on one bill (for counting exchanges).
fn bill_prefix(policy: u32) -> String {
    hooks::choice_id(HOOK.owner, &format!("{policy}."))
}

/// What an argument would do: shown before it is chosen, applied as shown.
#[derive(Clone, Debug, PartialEq)]
pub struct Pitch {
    pub argument: Argument,
    pub actor: PoliticianId,
    pub target: PoliticianId,
    pub policy: u32,
    /// The side the actor lobbies for: +1 for the bill, −1 against.
    pub side: f64,
    /// The chance it works; 1 for an argument that cannot fail. The roll
    /// uses exactly this.
    pub odds: f64,
    /// Sway on success: uniform in `gain` × `scale` (a favour: `gain[0]`).
    pub gain: [f64; 2],
    pub scale: f64,
    /// Sway when it fails.
    pub fail_gain: f64,
    /// The target's opinion of the actor after a success and a failure.
    pub opinion: [f64; 2],
    /// A back-room deal: the late slot, health and legal jeopardy.
    pub late: bool,
    pub legal: f64,
    pub health: f64,
    /// The actor owes the target a favour for it.
    pub owe: bool,
    /// A favour the target owes the actor is called in.
    pub calls_in: bool,
    /// Bringing others: the sway each other member gains on success, and
    /// the yes votes the whip expects that to add.
    pub others: Vec<(PoliticianId, f64)>,
    pub votes: f64,
}

/// The member a character is, if they hold a post.
pub fn member_of(state: &State, c: CharacterId) -> Option<PoliticianId> {
    state.politics.characters.get(c)?.politician
}

/// A member is settled on a bill: its sponsor, or committed to a vote.
pub fn settled(state: &State, target: PoliticianId, policy: u32) -> bool {
    let sponsor = state
        .policy
        .instances
        .get(policy as usize)
        .and_then(|i| i.sponsor);
    sponsor == Some(target)
        || state
            .politics
            .declared_votes
            .contains_key(&(target.0, policy))
}

/// The side the player is on: their own bill, else their declared vote,
/// else for it.
pub fn player_side(state: &State, policy: u32) -> f64 {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return 1.0;
    };
    let own = state
        .policy
        .instances
        .get(policy as usize)
        .is_some_and(|i| i.sponsor == Some(me));
    if own {
        return 1.0;
    }
    match state.politics.declared_votes.get(&(me.0, policy)) {
        Some(v) if *v < 0 => -1.0,
        _ => 1.0,
    }
}

/// Lobbying exchanges with this person on this bill this week.
pub fn exchanges_this_week(state: &State, speaker: CharacterId, policy: u32) -> u32 {
    let Some(pl) = state.player.as_ref() else {
        return 0;
    };
    let prefix = bill_prefix(policy);
    pl.talk
        .this_week(speaker, crate::talk::week_of(state.calendar.date))
        .filter(|e| e.topic.starts_with(&prefix))
        .count() as u32
}

/// The whip of a party in a bill's legislature: its longest-serving member
/// (ties: the lowest id), not `except`.
pub fn party_whip(
    state: &State,
    content: &ContentBundle,
    policy: u32,
    party: crate::politics::PartyId,
    except: PoliticianId,
) -> Option<PoliticianId> {
    let inst = state.policy.instances.get(policy as usize)?;
    let c = content.constitutions.get(&state.meta.constitution)?;
    crate::politics::legislature_of(&state.politics, c, inst.at())
        .iter()
        .filter_map(|id| state.politics.get(*id))
        .filter(|p| p.id != except && p.party == Some(party))
        .max_by(|a, b| {
            a.years_in_office
                .cmp(&b.years_in_office)
                .then(b.id.0.cmp(&a.id.0))
        })
        .map(|p| p.id)
}

/// Whether the target may be lobbied on this bill at all: a holder of a
/// post in the bill's jurisdiction (a member, or whoever signs it), not the
/// actor, not settled.
fn lobbyable(state: &State, actor: PoliticianId, target: PoliticianId, policy: u32) -> bool {
    let Some(inst) = state.policy.instances.get(policy as usize) else {
        return false;
    };
    let Some(t) = state.politics.get(target) else {
        return false;
    };
    inst.status == PolicyStatus::UnderConsideration
        && target != actor
        && !t.is_player
        && t.role.post().is_some_and(|p| p.at() == inst.at())
        && !settled(state, target, policy)
}

/// The lobbying logit's shared part and the success odds with `term` in
/// place of the ties (`npc::lobby_odds_between` is this with the ties).
pub fn odds_with(
    state: &State,
    content: &ContentBundle,
    actor: PoliticianId,
    target: PoliticianId,
    term: f64,
    late_night: bool,
) -> f64 {
    let (Some(me), Some(t)) = (state.politics.get(actor), state.politics.get(target)) else {
        return 0.0;
    };
    let l = &content.council.lobby;
    let skill = me.negotiation as f64 * l.skill_negotiation + me.oratory as f64 * l.skill_oratory;
    let x = (skill - 0.5) * l.skill_scale + t.pragmatism as f64 * l.pragmatism + l.bias + term
        - l.integrity * (t.integrity as f64 - 0.5)
        + if late_night { l.late_night } else { 0.0 }
        + crate::npc::lobby_knowledge_logit(state, content, actor, target);
    clamp(sigmoid(x), l.odds_min, l.odds_max)
}

/// The ties term of the plain pitch: the target's opinion of the player,
/// or, between NPCs, party ties.
pub fn ties(
    state: &State,
    content: &ContentBundle,
    actor: PoliticianId,
    target: PoliticianId,
) -> f64 {
    let (Some(me), Some(t)) = (state.politics.get(actor), state.politics.get(target)) else {
        return 0.0;
    };
    let l = &content.council.lobby;
    if me.is_player {
        characters::opinion_between(&state.politics, target, actor) as f64 / l.opinion_scale
    } else if me.party.is_some() && me.party == t.party {
        l.same_party
    } else {
        l.other_party
    }
}

fn character_of(state: &State, p: PoliticianId) -> Option<CharacterId> {
    state.politics.get(p).map(|x| x.character)
}

fn loyalty_of(state: &State, p: PoliticianId) -> f64 {
    character_of(state, p)
        .and_then(|c| state.politics.characters.get(c))
        .map(|c| c.loyalty as f64)
        .unwrap_or(0.5)
}

/// The member's ward's support for the bill (−1..1), if they sit for a ward.
fn ward_support(
    state: &State,
    content: &ContentBundle,
    target: PoliticianId,
    policy: u32,
) -> Option<f64> {
    let inst = state.policy.instances.get(policy as usize)?;
    let t = crate::policy::resolve(content, &state.policy, inst)?;
    let ward = state.politics.get(target)?.role.constituency()?;
    let hoods = crate::policy::ward_hoods(&state.world, ward);
    Some(crate::policy::public_support(
        t.template(),
        &state.population,
        &hoods,
    ))
}

/// The player knows the worries of a neighbourhood in the member's ward.
fn knows_ward(state: &State, target: PoliticianId) -> bool {
    let Some(ward) = state
        .politics
        .get(target)
        .and_then(|p| p.role.constituency())
    else {
        return false;
    };
    crate::policy::ward_hoods(&state.world, ward)
        .into_iter()
        .any(|h| {
            knowledge::knows(
                state,
                &knowledge::Subject::HoodConcerns {
                    hood: NeighborhoodId(h as u32),
                },
            ) >= Level::Known
        })
}

/// The broker of the bill's jurisdiction and their pull on it, when the
/// player knows who they are.
fn known_pull(
    state: &State,
    content: &ContentBundle,
    policy: u32,
) -> Option<crate::characters::town::BrokerPull> {
    let inst = state.policy.instances.get(policy as usize)?;
    let t = crate::policy::resolve(content, &state.policy, inst)?;
    let pull = broker_pull(
        content,
        &state.politics,
        &state.world,
        &state.population,
        t.template(),
        inst,
    )?;
    let m = town::leading_town(&state.world, &state.population, inst.at())?;
    (knowledge::knows(state, &knowledge::Subject::Broker { town: m }) >= Level::Known)
        .then_some(pull)
}

/// Whether the target brings others: an ally of the actor, or the whip of
/// the actor's party in the bill's legislature.
pub fn brings_others(
    state: &State,
    content: &ContentBundle,
    actor: PoliticianId,
    target: PoliticianId,
    policy: u32,
) -> bool {
    let (Some(a), Some(t)) = (character_of(state, actor), character_of(state, target)) else {
        return false;
    };
    if characters::bonds(&state.politics, t, a).contains(&Bond::Ally) {
        return true;
    }
    let party = state.politics.get(actor).and_then(|p| p.party);
    party.is_some_and(|party| {
        state.politics.get(target).and_then(|p| p.party) == Some(party)
            && party_whip(state, content, policy, party, actor) == Some(target)
    })
}

/// What an argument would do now, if it can be made (`None`: it cannot).
/// The player's arguments read what the player knows; an NPC makes the
/// plain pitch (`side`: +1 for the bill, −1 against).
pub fn pitch(
    state: &State,
    content: &ContentBundle,
    actor: PoliticianId,
    target: PoliticianId,
    policy: u32,
    argument: Argument,
    side: f64,
) -> Option<Pitch> {
    if !lobbyable(state, actor, target, policy) {
        return None;
    }
    let me = state.politics.get(actor)?;
    let (ac, tc) = (me.character, character_of(state, target)?);
    let lp = &content.council.lobby;
    let fx = &content.actions.lobby;
    let args = &fx.arguments;
    // An NPC's lobbying is a whole action; the player's a conversation,
    // which costs half a slot and moves half as much (`talk.pitch`).
    let share = if me.is_player {
        content.talk.pitch.sway_share
    } else {
        1.0
    };
    let mut p = Pitch {
        argument,
        actor,
        target,
        policy,
        side,
        odds: 1.0,
        gain: lp.gain,
        scale: (0.5 + me.negotiation as f64) * share,
        fail_gain: lp.fail_gain * share,
        opinion: [fx.opinion_success, fx.opinion_fail],
        late: false,
        legal: 0.0,
        health: 0.0,
        owe: false,
        calls_in: false,
        others: Vec::new(),
        votes: 0.0,
    };
    let ties = ties(state, content, actor, target);
    let odds = |term: f64, late: bool| odds_with(state, content, actor, target, term, late);
    match argument {
        Argument::Plain => p.odds = odds(ties, false),
        Argument::AfterHours => {
            if !me.is_player || state.player.as_ref().is_some_and(|x| x.late_night_used) {
                return None;
            }
            p.odds = odds(ties, true);
            p.late = true;
            p.legal = fx.late_night_legal;
            p.health = content.actions.late_night_health;
        }
        Argument::Ward => {
            if !me.is_player || !knows_ward(state, target) {
                return None;
            }
            let support = ward_support(state, content, target, policy)?;
            p.odds = odds(args.ward * support * side, false);
        }
        Argument::Party => {
            let party = state.politics.get(target)?.party?;
            let inst = state.policy.instances.get(policy as usize)?;
            let t = crate::policy::resolve(content, &state.policy, inst)?;
            let c = content.constitutions.get(&state.meta.constitution)?;
            let sponsor_party = inst
                .sponsor
                .and_then(|s| state.politics.get(s))
                .and_then(|s| s.party);
            let governing = crate::politics::executive_of(&state.politics, c, inst.at())
                .and_then(|e| state.politics.get(e))
                .and_then(|e| e.party);
            let stance = crate::politics::party_line::member_stance(
                &content.politics.party_line,
                &state.politics,
                Some(party),
                sponsor_party,
                governing,
                &t.template().signature_weights,
            )?;
            if !me.is_player || stance.line as f64 * side <= 0.0 {
                return None;
            }
            p.odds = odds(args.party * loyalty_of(state, target), false);
        }
        Argument::Word => {
            if !me.is_player || state.asks.history_of(Asker::Person { id: tc }) != History::Kept {
                return None;
            }
            let trust = characters::trust(&state.politics, content, tc, ac) as f64;
            p.odds = odds(args.word * (2.0 * trust - 1.0), false);
        }
        Argument::Broker => {
            let pull = if me.is_player {
                known_pull(state, content, policy)?
            } else {
                return None;
            };
            if pull.broker == tc || pull.pull * side <= 0.0 {
                return None;
            }
            p.odds = odds(
                args.broker * compliance(&state.politics, tc, pull.broker),
                false,
            );
        }
        Argument::CallIn => {
            if !me.is_player || characters::favours_owed(&state.politics, tc, ac) == 0 {
                return None;
            }
            p.gain = [fx.trade_gain; 2];
            p.scale = 1.0;
            p.calls_in = true;
            p.opinion = [0.0, 0.0];
        }
        Argument::Owe => {
            if !me.is_player
                || characters::favours_owed(&state.politics, ac, tc) as i32 >= MAX_FAVOR_DEBT
            {
                return None;
            }
            p.gain = [fx.trade_gain * content.talk.favour.sway_share; 2];
            p.scale = 1.0;
            p.owe = true;
            p.opinion = [fx.opinion_success; 2];
        }
        Argument::Rally => {
            if !me.is_player
                || !brings_others(state, content, actor, target, policy)
                || characters::favours_owed(&state.politics, ac, tc) as i32 >= MAX_FAVOR_DEBT
            {
                return None;
            }
            p.odds = odds(ties, false);
            p.owe = true;
            let inst = state.policy.instances.get(policy as usize)?;
            let c = content.constitutions.get(&state.meta.constitution)?;
            let t = crate::policy::resolve(content, &state.policy, inst)?;
            let leans = crate::policy::legislature_leans(
                &content.council,
                &content.politics.party_line,
                c,
                &state.world,
                &state.population,
                &state.politics,
                state.meta.seed,
                t.template(),
                inst,
                broker_pull(
                    content,
                    &state.politics,
                    &state.world,
                    &state.population,
                    t.template(),
                    inst,
                ),
            );
            let party = state.politics.get(target).and_then(|x| x.party);
            for (id, lean) in leans {
                if id == target || !lobbyable(state, actor, id, policy) {
                    continue;
                }
                let Some(oc) = character_of(state, id) else {
                    continue;
                };
                let like = characters::opinion(&state.politics, oc, tc) as f64 / 100.0;
                let follow =
                    if party.is_some() && state.politics.get(id).and_then(|x| x.party) == party {
                        loyalty_of(state, id)
                    } else {
                        0.0
                    };
                let amount = args.rally_sway * clamp(like + follow, 0.0, 1.0);
                if amount <= 0.0 {
                    continue;
                }
                p.votes += (crate::policy::yes_probability(&content.council, lean + amount * side)
                    - crate::policy::yes_probability(&content.council, lean))
                    * side;
                p.others.push((id, amount));
            }
            if p.others.is_empty() {
                return None;
            }
        }
    }
    if !argument.rolled() {
        p.odds = 1.0;
    }
    Some(p)
}

/// Roll a pitch (the actor's stream, as lobbying always has): whether it
/// worked and the sway it gains (before the side). An argument that cannot
/// fail draws nothing.
pub fn roll(rng: &mut mandate_core::SimRng, p: &Pitch) -> (bool, f64) {
    if !p.argument.rolled() {
        return (true, p.gain[0] * p.scale);
    }
    let success = rng.chance(p.odds);
    let gain = if success {
        rng.range_f64(p.gain[0], p.gain[1]) * p.scale
    } else {
        p.fail_gain
    };
    (success, gain)
}

/// Lobbying success odds shown to the player before acting: the plain
/// pitch's (the same formula as NPC deals, `npc::lobby_odds_between`).
pub fn lobby_odds(
    state: &State,
    content: &ContentBundle,
    target: PoliticianId,
    late_night: bool,
) -> f64 {
    let Some(pl) = state.player.as_ref() else {
        return 0.0;
    };
    crate::npc::lobby_odds_between(state, content, pl.politician, target, late_night)
}

fn name(state: &State, c: CharacterId) -> String {
    characters::profile(&state.politics, &state.world, c)
        .map(|p| p.full_name())
        .unwrap_or_default()
}

/// The costs a choice shows, as Fluent keys and arguments.
fn costs(state: &State, p: &Pitch, speaker: CharacterId) -> Vec<(String, Vec<(String, String)>)> {
    let who = || vec![("name".to_string(), name(state, speaker))];
    let mut out = Vec::new();
    if p.late {
        out.push((
            "lobby-cost-late".into(),
            vec![("health".into(), format!("{}", p.health.round() as i64))],
        ));
        out.push((
            "lobby-cost-legal".into(),
            vec![("legal".into(), format!("{:.1}", p.legal))],
        ));
    } else {
        out.push(("lobby-cost-time".into(), Vec::new()));
    }
    if p.owe {
        out.push(("lobby-cost-owe".into(), who()));
    }
    if p.calls_in {
        out.push(("lobby-cost-call-in".into(), who()));
    }
    if p.argument.rolled() && p.opinion[1] < 0.0 {
        out.push(("lobby-cost-fail".into(), who()));
    }
    out
}

/// The label's arguments.
fn args(
    state: &State,
    content: &ContentBundle,
    p: &Pitch,
    speaker: CharacterId,
) -> Vec<(String, String)> {
    let mut a = vec![("speaker".to_string(), name(state, speaker))];
    if let Some(t) = state
        .policy
        .instances
        .get(p.policy as usize)
        .and_then(|i| crate::policy::resolve(content, &state.policy, i))
    {
        a.push(("bill_key".into(), t.template().name_key.clone()));
    }
    a.push((
        "side".into(),
        if p.side < 0.0 { "against" } else { "for" }.into(),
    ));
    if p.argument == Argument::Broker
        && let Some(b) = known_pull(state, content, p.policy)
    {
        a.push(("broker".into(), name(state, b.broker)));
    }
    if p.argument == Argument::Rally {
        a.push(("members".into(), p.others.len().to_string()));
        a.push(("votes".into(), format!("{:.1}", p.votes)));
    }
    a
}

fn choice(state: &State, content: &ContentBundle, p: &Pitch, speaker: CharacterId) -> Choice {
    let w = p.argument.word();
    Choice {
        id: choice_id(p.policy, p.argument),
        label_key: format!("lobby-choice-{w}"),
        say_key: format!("lobby-say-{w}"),
        args: args(state, content, p, speaker),
        replies: vec![Reply {
            key: format!("lobby-reply-{w}"),
            when: Vec::new(),
            agrees: true,
        }],
        cost: if p.late {
            ChoiceCost::LateSlot
        } else {
            ChoiceCost::Day
        },
        odds: p.argument.rolled().then_some(p.odds),
        costs: costs(state, p, speaker),
        remote: true,
        bill: Some(p.policy),
    }
}

/// The bills before a body of the player's focus that this person may be
/// lobbied on now: not settled, with exchanges left this week.
pub fn open_bills(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<u32> {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return Vec::new();
    };
    let Some(target) = member_of(state, speaker) else {
        return Vec::new();
    };
    let focus = state.focus();
    let limit = content.actions.lobby.arguments.exchanges;
    state
        .policy
        .instances
        .iter()
        .filter(|i| i.status == PolicyStatus::UnderConsideration && focus.contains(&i.at()))
        .map(|i| i.id.0)
        .filter(|b| lobbyable(state, me, target, *b))
        .filter(|b| exchanges_this_week(state, speaker, *b) < limit)
        .collect()
}

/// Every argument the player can make to `speaker` now, bill by bill.
pub fn arguments(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Pitch> {
    let (Some(me), Some(target)) = (
        state.player.as_ref().map(|p| p.politician),
        member_of(state, speaker),
    ) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for b in open_bills(state, content, speaker) {
        let side = player_side(state, b);
        for a in Argument::ALL {
            if let Some(p) = pitch(state, content, me, target, b, a, side) {
                out.push(p);
            }
        }
    }
    out
}

fn offer(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    arguments(state, content, speaker)
        .iter()
        .map(|p| choice(state, content, p, speaker))
        .collect()
}

/// The argument was made: roll it at the odds shown, and apply what it does.
fn chosen(
    state: &mut State,
    content: &ContentBundle,
    choice: &Choice,
    speaker: CharacterId,
    _reply: &Reply,
) -> Done {
    let done = |key: &str, args: Vec<(String, String)>| Done {
        outcome_key: key.into(),
        args,
    };
    let (Some((policy, argument)), Some(me), Some(target)) = (
        parse_id(&choice.id),
        state.player.as_ref().map(|p| p.politician),
        member_of(state, speaker),
    ) else {
        return done("outcome-talk", Vec::new());
    };
    let side = player_side(state, policy);
    let Some(mut p) = pitch(state, content, me, target, policy, argument, side) else {
        return done("outcome-talk", Vec::new());
    };
    // The odds shown, fixed before the conversation taught anything.
    if let Some(o) = choice.odds {
        p.odds = o;
    }
    let rng = state.rng.get(Stream::Player);
    let (success, gain) = roll(rng, &p);
    apply(state, content, &p, success, gain);
    let mut args = vec![("name".to_string(), name(state, speaker))];
    args.extend(choice.args.iter().filter(|(k, _)| k == "bill_key").cloned());
    let key = match (argument, success) {
        (Argument::CallIn, _) => "outcome-lobby-called-in",
        (Argument::Owe, _) => "outcome-lobby-owe",
        (Argument::Rally, true) => {
            args.push(("members".into(), p.others.len().to_string()));
            "outcome-lobby-rallied"
        }
        (_, true) => "outcome-lobby-won",
        (_, false) => "outcome-lobby-lost",
    };
    done(key, args)
}

/// Apply a rolled pitch (the player's command): sway, favours, memory and
/// opinion, the record, what the player learns of the member, and a
/// back-room deal's legal jeopardy and hidden fact.
fn apply(state: &mut State, content: &ContentBundle, p: &Pitch, success: bool, gain: f64) {
    let date = state.calendar.date;
    let (Some(me), Some(tc)) = (character_of(state, p.actor), character_of(state, p.target)) else {
        return;
    };
    *state
        .politics
        .sway
        .entry((p.target.0, p.policy))
        .or_default() += (gain * p.side) as f32;
    if success {
        for (id, amount) in &p.others {
            *state.politics.sway.entry((id.0, p.policy)).or_default() += (amount * p.side) as f32;
        }
    }
    let subject = Some(Subject::Policy { id: p.policy });
    if p.calls_in {
        characters::repay(&mut state.politics, tc, me);
    }
    if p.owe && success {
        characters::owe(&mut state.politics, me, tc, "favour-bill", subject, date);
    }
    let kind = match (p.argument, success) {
        (Argument::Owe, _) => MemoryKind::PromisedFavour,
        (Argument::CallIn, _) | (_, false) => MemoryKind::PressedMe,
        (_, true) => MemoryKind::LobbiedMe,
    };
    let opinion = if success { p.opinion[0] } else { p.opinion[1] };
    characters::remember(
        &mut state.politics,
        content,
        date,
        tc,
        me,
        kind,
        subject,
        (opinion != 0.0).then_some(opinion),
    );
    crate::politics::add_record(
        &mut state.politics,
        p.actor,
        content.career.record_gain.lobby as f32,
    );
    // A second meeting, or one late at night, is a closer look (DEC-091).
    let deep = p.late
        || knowledge::knows(state, &knowledge::Subject::Person { person: tc }) >= Level::Detailed
        || state.player.as_ref().is_some_and(|x| {
            x.talk
                .log
                .iter()
                .filter(|e| e.speaker == tc && e.topic.starts_with(&bill_prefix(p.policy)))
                .count()
                >= 1
        });
    if deep {
        let src = knowledge::action_source(state, "lobby");
        knowledge::teach_one(
            state,
            knowledge::Subject::Person { person: tc },
            Level::Detailed,
            src,
        );
    }
    if p.late {
        if let Some(x) = state.player.as_mut() {
            // Back-room deals carry legal exposure (DEC-006).
            x.tracks.legal_jeopardy = (x.tracks.legal_jeopardy + p.legal as f32).min(100.0);
        }
        // A hidden fact (DEC-034): its fate is settled now; the director
        // times when it surfaces.
        crate::media::hide(
            state,
            content,
            &content.media.late_night_deal,
            p.actor,
            vec![crate::narrative::Actor::Person { id: tc }],
            Cause::Player(PlayerAct::Lobby),
        );
    }
}
