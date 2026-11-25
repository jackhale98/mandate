//! What knowing the power broker opens (M6 note 27; DEC-091's principle:
//! knowledge changes what the player can do, never the same option made
//! better). Once the player knows who the broker of a jurisdiction of
//! theirs is, a conversation with the broker offers three choices, and one
//! with the press a fourth (S4 talk hooks, owner `broker`):
//!
//! - **Lobby the broker** on a bill before the body that does not cut
//!   against their stake: one conversation in place of several, and a
//!   favour owed to the broker (or one they owe called in). Each member
//!   gains sway as far as they go along with the broker. The broker refuses
//!   below `lobby_min_opinion` of the player.
//! - **Do them a favour:** the broker owes the player one and thinks better
//!   of them; those who resent the broker think a little less of the player.
//! - **Oppose them openly:** popular with those who resent the broker,
//!   costly with those who depend on them; the broker and the members who
//!   like them cool on the player.
//! - **Expose them through the press:** the press's gaffe risk applies (the
//!   odds shown are the odds rolled). It works: members distance themselves
//!   from the broker (who then moves them less), those who resent the
//!   broker approve, and the press covers the player's statement. It
//!   misfires: approval falls. Either way the broker hears of it.
//!
//! [`preview`] is what the choice's label shows and exactly what [`chosen`]
//! applies (opinion changes before the holder's personality scales them).
//! Tier-generic (DEC-098): a broker per jurisdiction of the player's focus,
//! from live state (`characters::town::broker_of`). The player's command
//! writes the sway and favours it settles, as lobbying does; approval and
//! opinions go through effect requests with the player's act as their
//! cause. Facts and Fluent keys only.

use crate::characters::town::{self, broker_of, compliance};
use crate::characters::{self, CharacterId, MemoryKind};
use crate::player::knowledge::{self, Level, Subject};
use crate::policy::{PolicyStatus, yes_probability};
use crate::politics::PoliticianId;
use crate::scheduler::{Effect, Request};
use crate::spine::{Cause, PlayerAct};
use crate::state::State;
use crate::talk::hooks::{self, Choice, ChoiceCost, Done, Hook, Reply};
use crate::world::{Area, Jurisdiction};
use mandate_content::{Condition, ContentBundle};
use mandate_core::Stream;

/// The broker's hook (registered in `talk::hooks::HOOKS`).
pub const HOOK: Hook = Hook {
    owner: "broker",
    offer,
    chosen,
};

/// Keyed-RNG tag of an exposé's roll (DEC-041): never reused.
const KEY_EXPOSE: u64 = 0xB20C_E500;

/// What knowing the broker opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opening {
    Lobby { policy: u32 },
    Favour,
    Oppose,
    Expose,
}

impl Opening {
    /// `choice.broker.<local>`: the `Talk` command's `topic`.
    pub fn id(self) -> String {
        let local = match self {
            Opening::Lobby { policy } => format!("lobby.{policy}"),
            Opening::Favour => "favour".into(),
            Opening::Oppose => "oppose".into(),
            Opening::Expose => "expose".into(),
        };
        hooks::choice_id(HOOK.owner, &local)
    }

    pub fn parse(id: &str) -> Option<Opening> {
        let local = id.strip_prefix(hooks::PREFIX)?.strip_prefix("broker.")?;
        match local {
            "favour" => Some(Opening::Favour),
            "oppose" => Some(Opening::Oppose),
            "expose" => Some(Opening::Expose),
            _ => local
                .strip_prefix("lobby.")?
                .parse()
                .ok()
                .map(|policy| Opening::Lobby { policy }),
        }
    }

    fn word(self) -> &'static str {
        match self {
            Opening::Lobby { .. } => "lobby",
            Opening::Favour => "favour",
            Opening::Oppose => "oppose",
            Opening::Expose => "expose",
        }
    }
}

/// The brokers the player knows (`known` or better), one per jurisdiction
/// of their focus whose leading town's broker they have found.
pub fn known_brokers(state: &State) -> Vec<(Jurisdiction, CharacterId)> {
    let Some(pl) = state.player.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for at in pl.focus() {
        let Some(m) = town::leading_town(&state.world, &state.population, at) else {
            continue;
        };
        if knowledge::knows(state, &Subject::Broker { town: m }) < Level::Known {
            continue;
        }
        if let Some(b) = broker_of(&state.politics, &state.world, &state.population, at) {
            out.push((at, b));
        }
    }
    out
}

/// What an opening will do: the label shows it, and the choice applies it.
#[derive(Clone, Debug, PartialEq)]
pub struct Preview {
    pub broker: CharacterId,
    /// Lobbying: sway each member gains on the bill.
    pub sway: Vec<(PoliticianId, f64)>,
    /// Lobbying: yes votes the whip expects to gain.
    pub votes: f64,
    /// Lobbying: the player owes the broker a favour (else calls one in).
    pub owe: bool,
    /// A favour: the broker owes the player one.
    pub broker_owes: bool,
    /// Opinion changes (holder, toward, change before personality).
    pub opinions: Vec<(CharacterId, CharacterId, f64)>,
    /// Approval of the player in the jurisdiction: (change × room, classes).
    pub approval: Vec<(f64, Vec<u8>)>,
    /// An exposé: the gaffe risk (rolled as shown), and what a gaffe does.
    pub risk: Option<f64>,
    pub gaffe_approval: f64,
    /// Where it happens.
    pub at: Option<Jurisdiction>,
}

fn me(state: &State) -> Option<CharacterId> {
    characters::player_character(state)
}

fn edge_opinion(state: &State, holder: CharacterId, toward: CharacterId) -> f64 {
    state
        .politics
        .characters
        .edge(holder, toward)
        .map(|e| e.opinion as f64)
        .unwrap_or(0.0)
}

/// The members of a jurisdiction's legislature, as the constitution names it.
fn members(state: &State, content: &ContentBundle, at: Jurisdiction) -> Vec<PoliticianId> {
    content
        .constitutions
        .get(&state.meta.constitution)
        .map(|c| crate::politics::legislature_of(&state.politics, c, at).to_vec())
        .unwrap_or_default()
}

/// Whether the player stood against the broker (opposed or exposed them)
/// within `cooldown_days`.
fn stood_against_lately(state: &State, content: &ContentBundle, broker: CharacterId) -> bool {
    let Some(me) = me(state) else {
        return false;
    };
    let today = state.calendar.date;
    let days = content.characters.broker.cooldown_days;
    state.politics.characters.get(broker).is_some_and(|c| {
        c.memories.iter().any(|m| {
            m.actor == me && m.kind == MemoryKind::StoodAgainstMe && m.date.days_until(today) < days
        })
    })
}

/// What an opening would do now, if it is open (`None`: it is not).
pub fn preview(
    state: &State,
    content: &ContentBundle,
    opening: Opening,
    broker: CharacterId,
    at: Jurisdiction,
) -> Option<Preview> {
    let me = me(state)?;
    if broker == me {
        return None;
    }
    let k = &content.characters.broker;
    let (depend, resent) = town::depend_and_resent(content, &state.politics, broker);
    let mut p = Preview {
        broker,
        sway: Vec::new(),
        votes: 0.0,
        owe: false,
        broker_owes: false,
        opinions: Vec::new(),
        approval: Vec::new(),
        risk: None,
        gaffe_approval: 0.0,
        at: Some(at),
    };
    let liking = |m: PoliticianId| {
        state
            .politics
            .get(m)
            .map(|x| edge_opinion(state, x.character, broker) / 100.0)
            .unwrap_or(0.0)
    };
    match opening {
        Opening::Lobby { policy } => {
            let inst = state.policy.instances.get(policy as usize)?;
            if inst.status != PolicyStatus::UnderConsideration
                || broker_of(&state.politics, &state.world, &state.population, inst.at())
                    != Some(broker)
            {
                return None;
            }
            let t = crate::policy::resolve(content, &state.policy, inst)?;
            let t = t.template();
            // Not a bill that cuts against their stake.
            let m = town::leading_town(&state.world, &state.population, inst.at())?;
            let tc = state.politics.characters.towns.get(m.index())?;
            let stake = town::stake_of(content, &state.politics, tc, broker);
            if town::interest(content, t, inst, &stake) < 0.0 {
                return None;
            }
            let owed_by_broker = characters::favours_owed(&state.politics, broker, me) > 0;
            if !owed_by_broker
                && characters::favours_owed(&state.politics, me, broker) as i32
                    >= super::MAX_FAVOR_DEBT
            {
                return None;
            }
            p.owe = !owed_by_broker;
            let c = content.constitutions.get(&state.meta.constitution)?;
            let leans = crate::policy::legislature_leans(
                &content.council,
                &content.politics.party_line,
                c,
                &state.world,
                &state.population,
                &state.politics,
                state.meta.seed,
                t,
                inst,
                town::broker_pull(
                    content,
                    &state.politics,
                    &state.world,
                    &state.population,
                    t,
                    inst,
                ),
            );
            for (id, lean) in leans {
                let Some(x) = state.politics.get(id) else {
                    continue;
                };
                // The player votes for themselves; the sponsor's mind and a
                // declared vote are made up.
                if x.is_player
                    || inst.sponsor == Some(id)
                    || state.politics.declared_votes.contains_key(&(id.0, policy))
                {
                    continue;
                }
                let amount = k.lobby_sway * compliance(&state.politics, x.character, broker);
                if amount <= 0.0 {
                    continue;
                }
                p.votes += yes_probability(&content.council, lean + amount)
                    - yes_probability(&content.council, lean);
                p.sway.push((id, amount));
            }
            if p.sway.is_empty() {
                return None;
            }
        }
        Opening::Favour => {
            if characters::favours_owed(&state.politics, broker, me) as i32 >= super::MAX_FAVOR_DEBT
            {
                return None;
            }
            p.broker_owes = true;
            p.opinions.push((broker, me, k.favour_opinion));
            if !resent.is_empty() {
                p.approval.push((k.favour_resent_approval, resent));
            }
        }
        Opening::Oppose => {
            if stood_against_lately(state, content, broker) {
                return None;
            }
            p.opinions.push((broker, me, k.oppose_opinion));
            for m in members(state, content, at) {
                let Some(x) = state.politics.get(m) else {
                    continue;
                };
                let like = liking(m);
                if x.is_player || x.character == broker || like <= 0.0 {
                    continue;
                }
                p.opinions
                    .push((x.character, me, k.oppose_ally_opinion * like));
            }
            if !resent.is_empty() {
                p.approval.push((k.oppose_resent_approval, resent));
            }
            if !depend.is_empty() {
                p.approval.push((k.oppose_depend_approval, depend));
            }
        }
        Opening::Expose => {
            if stood_against_lately(state, content, broker) {
                return None;
            }
            p.risk = Some(super::press_gaffe_risk(state, content));
            p.gaffe_approval = k.expose_gaffe_approval;
            // On success; the broker's own anger is applied either way.
            p.opinions.push((broker, me, k.expose_opinion));
            for m in members(state, content, at) {
                let Some(x) = state.politics.get(m) else {
                    continue;
                };
                if x.is_player || x.character == broker {
                    continue;
                }
                p.opinions
                    .push((x.character, broker, k.expose_member_opinion));
            }
            if !resent.is_empty() {
                p.approval.push((k.expose_resent_approval, resent));
            }
        }
    }
    Some(p)
}

fn name(state: &State, c: CharacterId) -> String {
    characters::profile(&state.politics, &state.world, c)
        .map(|p| p.full_name())
        .unwrap_or_default()
}

/// The label's arguments: who, what, and the price in numbers.
fn args(
    state: &State,
    content: &ContentBundle,
    o: Opening,
    p: &Preview,
    speaker: CharacterId,
) -> Vec<(String, String)> {
    let mut a = vec![
        ("speaker".into(), name(state, speaker)),
        ("broker".into(), name(state, p.broker)),
    ];
    if let Opening::Lobby { policy } = o
        && let Some(t) = state
            .policy
            .instances
            .get(policy as usize)
            .and_then(|i| crate::policy::resolve(content, &state.policy, i))
    {
        a.push(("bill_key".into(), t.template().name_key.clone()));
        a.push(("members".into(), p.sway.len().to_string()));
        a.push(("votes".into(), format!("{:.1}", p.votes)));
        a.push(("price".into(), if p.owe { "owe" } else { "call_in" }.into()));
    }
    if let Some(r) = p.risk {
        a.push(("risk".into(), format!("{}", (r * 100.0).round() as i64)));
    }
    a
}

/// The replies: lobbying is refused when the broker thinks too little of
/// the player; the rest are said as they are.
fn replies(content: &ContentBundle, o: Opening) -> Vec<Reply> {
    let w = o.word();
    let yes = Reply {
        key: format!("broker-reply-{w}"),
        when: Vec::new(),
        agrees: true,
    };
    match o {
        Opening::Lobby { .. } => vec![
            Reply {
                key: "broker-reply-lobby".into(),
                when: vec![Condition::Opinion {
                    slot: "speaker".into(),
                    min: Some(content.characters.broker.lobby_min_opinion),
                    max: None,
                }],
                agrees: true,
            },
            Reply {
                key: "broker-reply-lobby-no".into(),
                when: Vec::new(),
                agrees: false,
            },
        ],
        _ => vec![yes],
    }
}

fn choice(
    state: &State,
    content: &ContentBundle,
    o: Opening,
    p: &Preview,
    speaker: CharacterId,
) -> Choice {
    let w = o.word();
    Choice {
        id: o.id(),
        label_key: format!("broker-choice-{w}"),
        say_key: format!("broker-say-{w}"),
        args: args(state, content, o, p, speaker),
        replies: replies(content, o),
        cost: ChoiceCost::Conversation,
    }
}

/// The openings for a conversation with `speaker`: with a broker the player
/// knows, lobbying (a bill at a time), a favour and open opposition; with
/// the press, an exposé of a broker the player knows.
fn offer(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    let mut out = Vec::new();
    let known = known_brokers(state);
    for (at, b) in &known {
        if *b != speaker {
            continue;
        }
        for (i, inst) in state.policy.instances.iter().enumerate() {
            if inst.status != PolicyStatus::UnderConsideration {
                continue;
            }
            let o = Opening::Lobby { policy: i as u32 };
            if let Some(p) = preview(state, content, o, *b, *at) {
                out.push(choice(state, content, o, &p, speaker));
            }
        }
        for o in [Opening::Favour, Opening::Oppose] {
            if let Some(p) = preview(state, content, o, *b, *at) {
                out.push(choice(state, content, o, &p, speaker));
            }
        }
    }
    let press = crate::visits::occupation_of(state, speaker).is_some_and(|o| {
        content
            .characters
            .broker
            .press_occupations
            .iter()
            .any(|x| x == o)
    });
    if press
        && let Some((at, b)) = known.iter().find(|(_, b)| *b != speaker)
        && let Some(p) = preview(state, content, Opening::Expose, *b, *at)
    {
        out.push(choice(state, content, Opening::Expose, &p, speaker));
    }
    out
}

/// The broker an opening offered to `speaker` is about.
fn broker_for(
    state: &State,
    o: Opening,
    speaker: CharacterId,
) -> Option<(Jurisdiction, CharacterId)> {
    let known = known_brokers(state);
    match o {
        Opening::Expose => known.into_iter().find(|(_, b)| *b != speaker),
        _ => known.into_iter().find(|(_, b)| *b == speaker),
    }
}

/// Whether an exposé misfires: a keyed draw per turn and broker (DEC-041),
/// so it advances no stream and the odds shown are the odds rolled.
pub fn expose_misfires(state: &State, broker: CharacterId, risk: f64) -> bool {
    state
        .rng
        .keyed(
            Stream::Player,
            &[KEY_EXPOSE, state.calendar.turn as u64, broker.0 as u64],
        )
        .chance(risk)
}

/// Apply a preview: sway and favours directly (the player's command, as
/// lobbying does), opinions and approval through effect requests.
pub fn apply(state: &mut State, content: &ContentBundle, o: Opening, p: &Preview, misfired: bool) {
    let Some(me) = me(state) else {
        return;
    };
    let date = state.calendar.date;
    let cause = Cause::Player(PlayerAct::Talk);
    let mut reqs: Vec<Request> = Vec::new();
    let push = |reqs: &mut Vec<Request>, effect: Effect| reqs.push(Request { effect, cause });
    match o {
        Opening::Lobby { policy } => {
            for (id, amount) in &p.sway {
                *state.politics.sway.entry((id.0, policy)).or_default() += *amount as f32;
            }
            let subject = Some(characters::Subject::Policy { id: policy });
            if p.owe {
                characters::owe(
                    &mut state.politics,
                    me,
                    p.broker,
                    "favour-broker",
                    subject,
                    date,
                );
            } else {
                characters::repay(&mut state.politics, p.broker, me);
            }
            push(
                &mut reqs,
                Effect::Opinion {
                    holder: p.broker,
                    toward: me,
                    delta: 0.0,
                    memory: Some(MemoryKind::LobbiedMe),
                },
            );
        }
        Opening::Favour => {
            characters::owe(
                &mut state.politics,
                p.broker,
                me,
                "favour-broker",
                None,
                date,
            );
        }
        Opening::Oppose | Opening::Expose => {}
    }
    for (holder, toward, delta) in &p.opinions {
        let to_broker = *holder == p.broker;
        if o == Opening::Expose && misfired && !to_broker {
            continue;
        }
        let memory = match (o, to_broker) {
            (Opening::Favour, true) => Some(MemoryKind::StoodWithMe),
            (Opening::Oppose | Opening::Expose, true) => Some(MemoryKind::StoodAgainstMe),
            (Opening::Expose, false) => Some(MemoryKind::Scandal),
            _ => None,
        };
        push(
            &mut reqs,
            Effect::Opinion {
                holder: *holder,
                toward: *toward,
                delta: *delta,
                memory,
            },
        );
    }
    let scope = p.at.map(|at| Area::In { at });
    if let Some(scope) = scope {
        if o == Opening::Expose && misfired {
            push(
                &mut reqs,
                Effect::Approval {
                    scope,
                    delta: p.gaffe_approval,
                    classes: Vec::new(),
                    communities: Vec::new(),
                },
            );
        } else {
            for (delta, classes) in &p.approval {
                push(
                    &mut reqs,
                    Effect::Approval {
                        scope,
                        delta: *delta,
                        classes: classes.clone(),
                        communities: Vec::new(),
                    },
                );
            }
        }
    }
    crate::asks::effects::apply_now(state, content, reqs);
    if o == Opening::Expose
        && !misfired
        && let Some(pl) = state.player.as_ref().map(|p| p.politician)
    {
        // A story: the outlets decide next week how to cover it.
        crate::media::statement(state, content, pl, None);
    }
}

/// The choice was picked: apply what its preview said (a refusal does
/// nothing; an exposé rolls its shown risk first).
fn chosen(
    state: &mut State,
    content: &ContentBundle,
    choice: &Choice,
    speaker: CharacterId,
    reply: &Reply,
) -> Done {
    let fail = |key: &str| Done {
        outcome_key: key.into(),
        args: Vec::new(),
    };
    let Some(o) = Opening::parse(&choice.id) else {
        return fail("outcome-talk");
    };
    let Some((at, b)) = broker_for(state, o, speaker) else {
        return fail("outcome-talk");
    };
    let Some(p) = preview(state, content, o, b, at) else {
        return fail("outcome-talk");
    };
    let w = o.word();
    let args = vec![("broker".into(), name(state, b))];
    if !reply.agrees {
        return Done {
            outcome_key: format!("outcome-broker-{w}-refused"),
            args,
        };
    }
    let misfired = o == Opening::Expose && p.risk.is_some_and(|r| expose_misfires(state, b, r));
    apply(state, content, o, &p, misfired);
    Done {
        outcome_key: if misfired {
            "outcome-broker-expose-misfired".into()
        } else {
            format!("outcome-broker-{w}")
        },
        args,
    }
}
