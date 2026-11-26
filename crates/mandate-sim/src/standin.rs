//! Stand-in players (STAGED_ARCS §6.3; DEC-102 clauses 6 and 7): directive
//! sets on the skip proxy that play a career so that it can be measured.
//! The arc grader, a start's rating and the repetition measure all play
//! with them, and the audition will (task A7).
//!
//! - **Passive** ends each turn: defaults fire, asks lapse.
//! - **Engaged** takes every lever the arcs list ([`arcs::beats::offers`]),
//!   for the arc's ally (`sides.ally`), then keeps the seat with the proxy
//!   at its fullest directives.
//! - **Contrarian** does the same for the opponent, and declines the
//!   ally's asks.
//! - **Attends** (attends the stops) skips the quiet weeks with the
//!   player's own directives and, at every stop a skip makes, plays that
//!   turn as the engaged stand-in; a campaign it plays turn by turn. A
//!   scene on offer at its seat is a stop it meets: it does not skip past
//!   one.
//!
//! **Scenes** (DEC-112; EXCHANGE_SCENES §8.2). A stand-in plays an
//! exchange scene through its own commands (`Begin` at the cast place,
//! one `Say` a round, `Leave` only when nothing it would say is listed),
//! choosing each line from the scene's view as the player sees it
//! ([`crate::arcs::exchange::view`]: redacted by the ledger, DEC-091), by
//! a [`Play`]: *absent* never begins (the empty chair), *cautious* says
//! the line with the best expected change by weight, *bold* the one with
//! the widest swing. Every draw is the scene's own, keyed on the run, the
//! beat, the round and the line: a stand-in rolls the odds the view
//! shows, as a player does, and touches no other stream. The engaged and
//! contrarian stand-ins begin on the last turn they can (so the weeks
//! before are for preparing); the attending one at the stop. A passive
//! stand-in given a style other than absent turns up for the scene and
//! does nothing else. The skip's proxy still never plays one (DEC-102
//! clause 7).
//!
//! **Only commands.** A stand-in reads the state it is shown and returns
//! `PlayerCommand`s ([`Turn::next`]); a career is those commands, `EndTurn`
//! and `Skip` through [`Engine::apply`] ([`step`]). Nothing else is
//! written, so a game's command log replays to the same state.
//!
//! **Only what a player would know.** The levers are the ones the arcs
//! offer the player; an option is weighed by the effects its dilemma
//! states and the cast it names; the proxy reads gated facts through
//! `Knowing`. A stand-in draws nothing itself: the proxy's draws are keyed
//! (`proxy::KEY_PROXY`).
//!
//! **Tier-generic (DEC-098).** A stand-in plays for a [`Seat`]: a
//! jurisdiction and the post it holds or runs for there. It takes the
//! levers of the runs at that jurisdiction and names no office.
//!
//! [`rate`] is a start's rating (DEC-102 clause 6): the same race played
//! several times under different luck, as a win rate with the factors
//! that drive it. Its forks are the measurer's, not a stand-in's: a fork
//! is a copy with its luck re-keyed ([`reluck`]), played by commands, and
//! dropped.

use crate::actions::{Action, PlayerCommand};
use crate::arcs::beats::{self, BeatAction, BeatOffer};
use crate::arcs::exchange::{self, ExchangeAct, OptionView, Phase, SceneView};
use crate::asks::{Ask, Asker, Need, StatementNeed};
use crate::player::knowledge::Knowing;
use crate::politics::Post;
use crate::proxy::ProxyTurn;
use crate::skip::{Directives, SkipStop};
use crate::state::State;
use crate::world::Jurisdiction;
use crate::{Command, Engine};
use mandate_content::act::SkipTargetKind;
use mandate_content::arcs::{ArcEffect, BeatKind};
use mandate_content::exchange::MoveKind;
use mandate_content::{ContentBundle, EffectDef};
use mandate_core::Stream;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Keyed-draw tag of a fork's luck ([`reluck`]; `Stream::Misc`).
pub const KEY_LUCK: u64 = 0x57A4_D141;

/// At most this many commands a turn (levers, then the proxy's own cap).
const MAX_COMMANDS: u8 = 32;

/// Which stand-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Passive,
    Engaged,
    Contrarian,
    Attends,
}

impl Kind {
    /// The three the audition and the grader compare.
    pub const ARC: [Kind; 3] = [Kind::Passive, Kind::Engaged, Kind::Contrarian];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Passive => "passive",
            Kind::Engaged => "engaged",
            Kind::Contrarian => "contrarian",
            Kind::Attends => "attends",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "passive" => Kind::Passive,
            "engaged" => Kind::Engaged,
            "contrarian" => Kind::Contrarian,
            "attends" => Kind::Attends,
            _ => return None,
        })
    }

    /// How this kind plays a scene unless told otherwise (§8.2): the
    /// passive one is absent, the contrarian bold, the others cautious.
    pub fn play(self) -> Play {
        match self {
            Kind::Passive => Play::Absent,
            Kind::Contrarian => Play::Bold,
            Kind::Engaged | Kind::Attends => Play::Cautious,
        }
    }
}

/// How a stand-in plays an exchange scene (DEC-112; EXCHANGE_SCENES §8.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Play {
    /// Never begins: the other side speaks to an empty chair.
    Absent,
    /// Each round, the line with the best expected change by weight: each
    /// group's range at its middle, a line with a chance at its expected
    /// value. Never a line that airs a secret or denies; a promise only
    /// when it beats the best other line by [`PROMISE_MARGIN`]. Ties go
    /// to the table's order.
    Cautious,
    /// Each round, the line with the widest swing by weight: the gap
    /// between the best and the worst it can do, stood or not; ties to the
    /// larger best case, then the table's order. Never `own`; `pass` only
    /// when nothing else is listed.
    Bold,
}

impl Play {
    pub const ALL: [Play; 3] = [Play::Absent, Play::Cautious, Play::Bold];

    pub fn key(self) -> &'static str {
        match self {
            Play::Absent => "absent",
            Play::Cautious => "cautious",
            Play::Bold => "bold",
        }
    }

    pub fn parse(s: &str) -> Option<Play> {
        Play::ALL.into_iter().find(|p| p.key() == s)
    }
}

/// A stand-in's name with its play style: `engaged` (the kind's own
/// style) or `engaged/bold`.
pub fn parse_styled(s: &str) -> Option<(Kind, Play)> {
    match s.split_once('/') {
        Some((k, p)) => Some((Kind::parse(k)?, Play::parse(p)?)),
        None => Kind::parse(s).map(|k| (k, k.play())),
    }
}

/// The name [`parse_styled`] reads: the kind alone when the style is its
/// own.
pub fn styled_key(kind: Kind, play: Play) -> String {
    if play == kind.play() {
        kind.key().to_string()
    } else {
        format!("{}/{}", kind.key(), play.key())
    }
}

/// How much better (by weight, in lean) a promise must look than the best
/// other line for the cautious stand-in to spend one of the campaign's
/// few on it: a point (the stand-in's own rule, not the game's).
pub const PROMISE_MARGIN: f64 = 0.01;

/// A line's previewed change by weight: each group's range at its middle,
/// a line with a chance at its expected value.
pub fn expected(v: &SceneView, o: &OptionView) -> f64 {
    v.groups
        .iter()
        .enumerate()
        .map(|(g, x)| {
            let s = o.stands.get(g).map_or(0.0, |x| x.mid());
            let f = o.fails.get(g).map_or(s, |x| x.mid());
            f64::from(x.weight) * (o.chance * s + (1.0 - o.chance) * f)
        })
        .sum()
}

/// A line's swing by weight: the gap between the best and the worst it
/// can do (its ranges, stood or not), and its best case.
pub fn swing(v: &SceneView, o: &OptionView) -> (f64, f64) {
    let mut gap = 0.0;
    let mut best = 0.0;
    for (g, x) in v.groups.iter().enumerate() {
        let Some(s) = o.stands.get(g) else { continue };
        let f = o.fails.get(g).filter(|_| o.chance < 1.0).unwrap_or(s);
        gap += f64::from(x.weight) * (s.hi.max(f.hi) - s.lo.min(f.lo));
        best += f64::from(x.weight) * s.hi.max(f.hi);
    }
    (gap, best)
}

/// The first line that scores highest (ties stay with the earlier line).
fn first_max<'v>(
    v: &'v SceneView,
    keep: &dyn Fn(&OptionView) -> bool,
    score: &dyn Fn(&OptionView) -> (f64, f64),
) -> Option<&'v OptionView> {
    let mut best: Option<(&OptionView, (f64, f64))> = None;
    for o in v.options.iter().filter(|o| keep(o)) {
        let x = score(o);
        if best.is_none_or(|(_, b)| x.0 > b.0 || (x.0 == b.0 && x.1 > b.1)) {
            best = Some((o, x));
        }
    }
    best.map(|b| b.0)
}

/// The line a play style says this round (`None`: the style never plays,
/// or nothing is listed). Reads the view alone.
pub fn line(v: &SceneView, play: Play) -> Option<String> {
    match play {
        Play::Absent => None,
        Play::Cautious => {
            let safe = |o: &OptionView| {
                !o.notes
                    .iter()
                    .any(|n| n.key == "exchange-cost-aired" || n.key == "exchange-cost-denial")
            };
            let ev = |o: &OptionView| (expected(v, o), 0.0);
            let other = first_max(v, &|o| safe(o) && o.kind != MoveKind::Promise, &ev);
            let promise = first_max(v, &|o| safe(o) && o.kind == MoveKind::Promise, &ev);
            match (other, promise) {
                (Some(a), Some(p)) if expected(v, p) > expected(v, a) + PROMISE_MARGIN => {
                    Some(p.id.clone())
                }
                (Some(a), _) => Some(a.id.clone()),
                (None, Some(p)) => Some(p.id.clone()),
                (None, None) => v.options.first().map(|o| o.id.clone()),
            }
        }
        Play::Bold => first_max(
            v,
            &|o| !matches!(o.kind, MoveKind::Own | MoveKind::Pass),
            &|o| swing(v, o),
        )
        .or_else(|| v.options.iter().find(|o| o.kind == MoveKind::Pass))
        .or_else(|| v.options.first())
        .map(|o| o.id.clone()),
    }
}

/// What a stand-in plays for: a jurisdiction, and the post it holds or
/// runs for (none: out of office and not standing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seat {
    pub at: Jurisdiction,
    pub post: Option<Post>,
}

impl Seat {
    /// The player's own: their home jurisdiction, and the post their next
    /// election is for (the one filed for, else the one held).
    pub fn of_player(state: &State, content: &ContentBundle) -> Option<Seat> {
        let home = crate::player::home(state)?;
        Some(Seat {
            at: Jurisdiction::town(home),
            post: crate::skip::election_post(state, content),
        })
    }
}

/// A post by its constitution id (an office's or a body's), for the
/// player: at the jurisdiction of its tier that holds their home, and for
/// a seat, the district of that body their ward lies in (none: the body
/// sits at large). `Err` says why not: no such id, or nobody holds posts
/// of that tier yet (a tier the engine does not instantiate).
pub fn post_named(state: &State, content: &ContentBundle, id: &str) -> Result<Post, String> {
    use mandate_content::{PostKey, PostRef};
    let c = content
        .constitutions
        .get(&state.meta.constitution)
        .ok_or("no constitution")?;
    let pl = state.player.as_ref().ok_or("no player")?;
    let (tier, def) = c.post(id).ok_or_else(|| {
        let known: Vec<&str> = c
            .tiers
            .iter()
            .flat_map(|t| {
                t.bodies
                    .iter()
                    .map(|b| b.id.as_str())
                    .chain(t.offices.iter().map(|o| o.id.as_str()))
            })
            .collect();
        format!(
            "no post `{id}` (the constitution has: {})",
            known.join(", ")
        )
    })?;
    // Up from the home town to the jurisdiction of the post's tier.
    let mut at = Some(Jurisdiction::town(pl.home()));
    while at.is_some_and(|j| j.tier() != tier) {
        at = at.and_then(|j| j.parent(&state.world));
    }
    let at = at.ok_or("the player's home is in no jurisdiction of that tier")?;
    let key = PostKey::new(id).ok_or("a post id is too long")?;
    let pol = &state.politics;
    match def {
        PostRef::Office(_) => {
            if !pol.offices.iter().any(|o| o.at == at && o.office == key) {
                return Err(format!(
                    "nobody holds `{id}` there: its tier is not elected yet"
                ));
            }
            Ok(Post::Office { office: key, at })
        }
        PostRef::Body(_) => {
            if !pol.bodies.iter().any(|b| b.at == at && b.body == key) {
                return Err(format!(
                    "nobody sits in `{id}` there: its tier is not elected yet"
                ));
            }
            let ward = crate::policy::ward_hoods(&state.world, pl.ward);
            let constituency = at
                .constituencies(&state.world)
                .find(|x| x.neighborhoods.iter().any(|h| ward.contains(&h.index())))
                .map(|x| x.id);
            Ok(Post::Seat {
                body: key,
                at,
                constituency,
            })
        }
    }
}

/// The fullest directive set the content offers: every effort directive
/// at its heaviest option, the others at their default.
pub fn full_directives(content: &ContentBundle) -> Directives {
    let mut choices: Vec<(String, String)> = Vec::new();
    for d in &content.act_flow.directives {
        let heaviest = d
            .options
            .iter()
            .filter_map(|o| o.weight.map(|w| (w, o.id.as_str())))
            // The first of the heaviest, in listed order.
            .fold(None::<(u8, &str)>, |best, x| match best {
                Some(b) if b.0 >= x.0 => Some(b),
                _ => Some(x),
            });
        if let Some((_, id)) = heaviest {
            choices.push((d.id.clone(), id.to_string()));
        }
    }
    choices.sort();
    Directives { choices }
}

/// Whether the player is a candidate in a running race.
pub fn campaigning(state: &State) -> bool {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return false;
    };
    state
        .politics
        .elections
        .races
        .iter()
        .any(|r| r.candidates.iter().any(|c| c.politician == me))
}

#[derive(Clone, Debug, PartialEq)]
enum Sent {
    Lever,
    /// A scene's `Begin`, `Say` or `Leave`.
    Scene,
    /// A bill drafted for an ask's need: introduce what it made.
    Draft,
    Proxy,
}

/// One turn of a stand-in. Use as `ProxyTurn`: ask [`Turn::next`] for a
/// command, apply it, report with [`Turn::result`], stop at `None`.
#[derive(Clone, Debug)]
pub struct Turn {
    kind: Kind,
    play: Play,
    seat: Seat,
    /// A scene's commands refused this turn: after one, `Leave`; after
    /// two, the scene is left alone.
    scene_refused: u8,
    /// Levers already sent this turn (done or refused): never twice.
    tried: Vec<String>,
    proxy: Option<ProxyTurn>,
    last: Option<Sent>,
    drafted: Option<u32>,
    sent: u8,
}

impl Turn {
    /// A turn of `kind` for `seat`. `Attends` plays a turn as `Engaged`
    /// (its skipping is [`step`]'s).
    pub fn new(kind: Kind, seat: Seat, content: &ContentBundle) -> Self {
        Turn::styled(kind, kind.play(), seat, content)
    }

    /// A turn of `kind` that plays scenes by `play`.
    pub fn styled(kind: Kind, play: Play, seat: Seat, content: &ContentBundle) -> Self {
        let proxy = match kind {
            Kind::Passive => None,
            _ => Some(ProxyTurn::with(crate::skip::settings(
                &full_directives(content),
                content,
            ))),
        };
        Turn {
            kind,
            play,
            seat,
            scene_refused: 0,
            tried: Vec::new(),
            proxy,
            last: None,
            drafted: None,
            sent: 0,
        }
    }

    /// How the last command went (`created`: the id it made, a draft's).
    pub fn result(&mut self, ok: bool, created: Option<u32>) {
        match self.last.take() {
            Some(Sent::Draft) if ok => self.drafted = created,
            Some(Sent::Scene) if !ok => self.scene_refused += 1,
            Some(Sent::Proxy) => {
                if let Some(p) = self.proxy.as_mut() {
                    p.result(ok, created);
                }
            }
            _ => {}
        }
    }

    /// The next command, or `None` when the turn's work is done.
    pub fn next(&mut self, st: &State, content: &ContentBundle) -> Option<PlayerCommand> {
        if self.sent >= MAX_COMMANDS {
            return None;
        }
        let pl = st.player.as_ref()?;
        // A scene first: one under way is played to its end (it never
        // spans a turn), and one due is begun while the turn has the time.
        if pl.status.active()
            && let Some(c) = self.scene(st, content)
        {
            self.sent += 1;
            self.last = Some(Sent::Scene);
            return Some(c);
        }
        if self.kind == Kind::Passive {
            return None;
        }
        if pl.status.active() {
            if let Some(id) = self.drafted.take() {
                self.sent += 1;
                self.last = Some(Sent::Lever);
                return Some(crate::places::routed(
                    st,
                    content,
                    PlayerCommand::IntroducePolicy { policy: id },
                ));
            }
            if let Some(c) = self.lever(st, content) {
                self.sent += 1;
                return Some(c);
            }
        }
        // Then the seat's upkeep, by the fullest directives.
        let c = self.proxy.as_mut()?.next(st, content)?;
        self.sent += 1;
        self.last = Some(Sent::Proxy);
        Some(c)
    }

    /// The scene's next command: a line in one under way at the seat's
    /// jurisdiction, else the `Begin` of one due now. Reads the scene's
    /// view, as the player has it.
    fn scene(&mut self, st: &State, content: &ContentBundle) -> Option<PlayerCommand> {
        if self.play == Play::Absent || self.scene_refused >= 2 {
            return None;
        }
        let at = self.seat.at;
        let under_way = st
            .arcs
            .runs
            .iter()
            .filter(|r| r.at == at)
            .find_map(|r| r.exchange.as_ref().map(|x| (r.id.0, x.beat.key())));
        if let Some((run, beat)) = under_way {
            let v = exchange::view(st, content, run, &beat)?;
            if v.phase != Phase::UnderWay {
                return None;
            }
            let act = match line(&v, self.play).filter(|_| self.scene_refused == 0) {
                Some(option) => ExchangeAct::Say { option },
                None => ExchangeAct::Leave,
            };
            return Some(PlayerCommand::Exchange { run, beat, act });
        }
        for (run, bref, location) in exchange::view::open_scenes(st, content) {
            let Some(r) = st.arcs.run(run).filter(|r| r.at == at) else {
                continue;
            };
            let key = format!("scene.{}.{}", run.0, bref.key());
            if self.tried.contains(&key) || !self.scene_due(st, content, r) {
                continue;
            }
            self.tried.push(key);
            return Some(PlayerCommand::Go {
                location,
                then: Some(Box::new(PlayerCommand::Exchange {
                    run: run.0,
                    beat: bref.key(),
                    act: ExchangeAct::Begin,
                })),
            });
        }
        None
    }

    /// Whether a scene on offer is begun this turn: at once by the one
    /// that attends the stops (and the passive one that only turns up);
    /// by the others on the last turn that can, so that the weeks before
    /// are for preparing.
    fn scene_due(&self, st: &State, content: &ContentBundle, run: &crate::arcs::ArcRun) -> bool {
        if matches!(self.kind, Kind::Attends | Kind::Passive) {
            return true;
        }
        let Some(last) = exchange::last_day(run) else {
            return true;
        };
        let days = crate::characters::routines::turn_days(st, content);
        st.calendar.date.add_days(days) > last
    }

    /// The next lever of the arcs at the seat's jurisdiction not yet tried
    /// this turn: a visit, a word with a cast person, an answer, an ask
    /// answered and then worked.
    fn lever(&mut self, st: &State, content: &ContentBundle) -> Option<PlayerCommand> {
        let offers: Vec<BeatOffer> = beats::offers(st, content)
            .into_iter()
            .filter(|o| st.arcs.run(o.run).is_some_and(|r| r.at == self.seat.at))
            .collect();
        for o in offers {
            let key = format!("{}.{}", o.run.0, o.beat.key());
            if self.tried.contains(&key) {
                continue;
            }
            self.tried.push(key);
            self.last = Some(Sent::Lever);
            let cmd = match &o.action {
                BeatAction::Visit { verb, location } => Some(PlayerCommand::Go {
                    location: *location,
                    then: Some(Box::new(act(Action::Visit { verb: verb.clone() }))),
                }),
                BeatAction::Talk { person, topic } => talk(st, content, *person, topic),
                BeatAction::Decision { decision } => {
                    answer(st, content, self.kind, *decision).map(|option| PlayerCommand::Answer {
                        decision: *decision,
                        option,
                    })
                }
                BeatAction::Ask { ask } => st
                    .asks
                    .asks
                    .iter()
                    .find(|a| a.id.0 == *ask && a.offered())
                    .map(|a| PlayerCommand::RespondAsk {
                        ask: *ask,
                        accept: self.kind != Kind::Contrarian || !from_ally(st, content, a),
                    }),
                // A scene (DEC-112) is begun by `Turn::scene`, on its day,
                // by the play style.
                BeatAction::Exchange { .. } => None,
            };
            if cmd.is_some() {
                return cmd;
            }
        }
        // What was promised is worked, soonest due first.
        let mut accepted: Vec<&Ask> = st.asks.asks.iter().filter(|a| a.accepted()).collect();
        accepted.sort_by_key(|a| (a.due, a.id));
        for a in accepted {
            let key = format!("ask.{}.{}", a.id.0, a.step);
            if self.tried.contains(&key) {
                continue;
            }
            self.tried.push(key);
            if let Some((cmd, sent)) = work(st, content, a) {
                self.last = Some(sent);
                return Some(cmd);
            }
        }
        None
    }
}

fn act(action: Action) -> PlayerCommand {
    PlayerCommand::Act {
        action,
        late_night: false,
    }
}

/// Go to where someone is and raise a topic with them (nothing when they
/// are nowhere the player can go this turn).
fn talk(
    st: &State,
    content: &ContentBundle,
    person: crate::characters::CharacterId,
    topic: &str,
) -> Option<PlayerCommand> {
    let location = crate::characters::routines::whereabouts(st, content, person)
        .filter(|l| crate::places::validate_move(st, *l).is_ok())?;
    Some(PlayerCommand::Go {
        location,
        then: Some(Box::new(PlayerCommand::Talk {
            to: crate::talk::TalkTarget::Person { person },
            topic: topic.to_string(),
            about: None,
            late_night: false,
        })),
    })
}

/// Whether an arc's ask comes from its ally.
fn from_ally(st: &State, content: &ContentBundle, a: &Ask) -> bool {
    let Some(r) = a.arc.as_ref().and_then(|x| st.arcs.run(x.run)) else {
        return false;
    };
    let Some(ally) = content
        .arc(&r.template)
        .and_then(|d| d.sides.as_ref())
        .and_then(|s| r.person(&s.ally))
    else {
        return false;
    };
    a.asker == Asker::Person { id: ally }
}

/// The option a stand-in takes in an arc's decision: a lever (never the
/// default, which is what not answering gives), the one whose stated
/// effects leave the ally thinking best of the player and the opponent
/// worst (`Engaged`), or the reverse (`Contrarian`). Ties go to an option
/// that sets a mark, then to the first listed (the last, for the
/// contrarian, so that the two part where nothing else tells them apart).
pub fn answer(st: &State, content: &ContentBundle, kind: Kind, decision: u32) -> Option<String> {
    let d = st
        .narrative
        .decision(crate::narrative::DecisionId(decision))?;
    let def = content.dilemma(&d.template)?;
    let arc = d.arc.as_ref()?;
    let run = st.arcs.run(arc.run)?;
    let adef = content.arc(&run.template)?;
    let on = match crate::arcs::beat_of(adef, &arc.beat).map(|(_, b)| &b.kind) {
        Some(BeatKind::Decision { on, .. }) => Some(on),
        _ => None,
    };
    let (ally, opponent) = adef
        .sides
        .as_ref()
        .map(|s| (s.ally.as_str(), s.opponent.as_str()))
        .unwrap_or(("", ""));
    let side = |role: &str, delta: f64| {
        if role == ally {
            delta
        } else if role == opponent {
            -delta
        } else {
            0.0
        }
    };
    let levers: Vec<&mandate_content::OptionDef> =
        def.options.iter().filter(|o| !o.default).collect();
    if levers.is_empty() {
        return def.options.first().map(|o| o.id.clone());
    }
    let n = levers.len();
    let mut scored: Vec<(f64, bool, usize, &str)> = levers
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let outcome = on.and_then(|m| m.get(&o.id));
            let own: f64 = o
                .effects
                .iter()
                .map(|e| match e {
                    EffectDef::Opinion { role, delta, .. } => side(role, *delta),
                    _ => 0.0,
                })
                .sum();
            let arcs: f64 = outcome
                .map(|x| {
                    x.effects
                        .iter()
                        .map(|g| match &g.effect {
                            ArcEffect::Opinion { role, delta, .. } => side(role, *delta),
                            _ => 0.0,
                        })
                        .sum()
                })
                .unwrap_or(0.0);
            let sets = outcome.is_some_and(|x| x.sets.is_some());
            let (score, order) = match kind {
                Kind::Contrarian => (-(own + arcs), n - i),
                _ => (own + arcs, i),
            };
            (score, sets, order, o.id.as_str())
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    scored.first().map(|x| x.3.to_string())
}

/// The command that meets an accepted ask's current need, if a command
/// does (a need kept by not acting has none).
fn work(st: &State, content: &ContentBundle, a: &Ask) -> Option<(PlayerCommand, Sent)> {
    let pl = st.player.as_ref()?;
    let route = |c: PlayerCommand| crate::places::routed(st, content, c);
    let cmd = match a.current()? {
        Need::Vote { policy, vote } => route(PlayerCommand::CastVote {
            policy: *policy,
            vote: *vote,
        }),
        Need::Statement { about } => route(act(Action::Press {
            policy: match about {
                StatementNeed::Bill { policy } => Some(*policy),
                _ => None,
            },
        })),
        Need::TownHall { hoods } => {
            let h = hoods.iter().find(|h| !a.done.contains(h))?;
            route(act(Action::TownHall {
                neighborhood: h.0,
                policy: crate::proxy::own_bill(st),
            }))
        }
        Need::Canvass { hood, .. } => route(act(Action::Canvass {
            neighborhood: hood.0,
            issue: Knowing::new(st, content).top_worry(*hood),
        })),
        Need::Visit { place } => PlayerCommand::MoveTo {
            location: crate::places::Location::Place { place: *place },
        },
        Need::Talk { person } => talk(st, content, *person, &crate::asks::hook::choice_id(a.id))?,
        Need::Introduce { templates } | Need::Enacted { templates, .. } => {
            let me = pl.politician;
            // One of these is already before the body: nothing to draft.
            if st.policy.instances.iter().any(|p| {
                p.sponsor == Some(me)
                    && p.status == crate::policy::PolicyStatus::UnderConsideration
                    && templates.contains(&p.template)
            }) {
                return None;
            }
            let t = templates
                .iter()
                .filter_map(|t| content.policies.get(t.as_str()))
                .find(|t| crate::policy::unlocked(st, content, t, pl.home().0))?;
            return Some((
                route(PlayerCommand::DraftPolicy {
                    template: t.id.clone(),
                    params: BTreeMap::new(),
                }),
                Sent::Draft,
            ));
        }
        Need::Casework => route(act(Action::Casework { ask: a.id.0 })),
        _ => return None,
    };
    Some((cmd, Sent::Lever))
}

// --- A career, step by step -------------------------------------------------------------

/// What a stand-in's career has come to (the measurer's memory, never
/// saved).
#[derive(Clone, Debug)]
pub struct Career {
    pub kind: Kind,
    /// How it plays a scene (the kind's own style unless told otherwise).
    pub play: Play,
    /// The seat it plays for; `None`: the player's own, read each turn.
    pub seat: Option<Seat>,
    /// `Attends`: a skip just stopped, and the next turn is played.
    attend: bool,
    pub played: u32,
    pub skipped: u32,
    /// The skips it made: how many turns each ran, and why it stopped
    /// (an interrupt's id, or `reached`, `limit`, `act_ended`).
    pub skips: Vec<(u32, String)>,
    pub commands: u32,
}

impl Career {
    pub fn new(kind: Kind) -> Self {
        Career::styled(kind, kind.play())
    }

    /// A career of `kind` that plays scenes by `play`.
    pub fn styled(kind: Kind, play: Play) -> Self {
        Career {
            kind,
            play,
            seat: None,
            attend: false,
            played: 0,
            skipped: 0,
            skips: Vec::new(),
            commands: 0,
        }
    }

    /// A career by its name ([`parse_styled`]: `engaged`, `engaged/bold`).
    pub fn parse(s: &str) -> Option<Self> {
        parse_styled(s).map(|(k, p)| Career::styled(k, p))
    }

    /// Its name, as [`Career::parse`] reads it.
    pub fn key(&self) -> String {
        styled_key(self.kind, self.play)
    }
}

/// What a step did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stepped {
    /// A turn was played and ended.
    Played,
    /// A stretch was skipped: the turns it ran.
    Skipped(u32),
    /// The career is over: no turn can end.
    Over,
}

/// The id a skip's stop is counted under.
pub fn stop_key(stop: &SkipStop) -> String {
    match stop {
        SkipStop::Reached => "reached".into(),
        SkipStop::Interrupted { interrupt, .. } => interrupt.clone(),
        SkipStop::Cancelled => "cancelled".into(),
        SkipStop::ActEnded { .. } => "act_ended".into(),
        SkipStop::Limit => "limit".into(),
    }
}

/// Send one turn's commands of a stand-in (the turn is not ended).
/// Returns how many the engine took.
pub fn play_turn(e: &mut Engine, kind: Kind, seat: Seat) -> u32 {
    play_turn_styled(e, kind, kind.play(), seat)
}

/// [`play_turn`] with a play style for scenes.
pub fn play_turn_styled(e: &mut Engine, kind: Kind, play: Play, seat: Seat) -> u32 {
    let mut t = Turn::styled(kind, play, seat, e.content());
    let mut sent = 0;
    while let Some(cmd) = t.next(e.state(), e.content()) {
        let r = e.apply(Command::Player(cmd));
        let created = r
            .as_ref()
            .ok()
            .and_then(|o| o.result.as_ref())
            .and_then(|x| x.created);
        sent += r.is_ok() as u32;
        t.result(r.is_ok(), created);
    }
    sent
}

/// The quiet stretch a skip would cover now: to the campaign, else to the
/// end of the term (none in a campaign, in prison, or after the act).
pub fn quiet_target(state: &State, content: &ContentBundle) -> Option<SkipTargetKind> {
    if campaigning(state) {
        return None;
    }
    let targets = crate::skip::targets(state, content);
    [SkipTargetKind::Campaign, SkipTargetKind::EndOfTerm]
        .into_iter()
        .find(|t| targets.iter().any(|x| x.target == *t))
}

/// Whether a scene at a jurisdiction is on offer or under way: a stop the
/// attending stand-in meets with a played turn, whether a skip stopped for
/// it or it opened in a turn it played (a skip begun with a scene already
/// on offer does not stop for it again).
pub fn scene_at(state: &State, content: &ContentBundle, at: Jurisdiction) -> bool {
    state
        .arcs
        .runs
        .iter()
        .any(|r| r.at == at && r.exchange.is_some())
        || exchange::view::open_scenes(state, content)
            .iter()
            .any(|(run, _, _)| state.arcs.run(*run).is_some_and(|r| r.at == at))
}

/// One step of a stand-in's career: a turn's commands and `EndTurn`; or,
/// for `Attends` in a quiet stretch with no stop to meet, one skip to its
/// stop.
pub fn step(e: &mut Engine, c: &mut Career) -> Stepped {
    let seat = c.seat.or_else(|| Seat::of_player(e.state(), e.content()));
    let meets_scene = c.kind == Kind::Attends
        && c.play != Play::Absent
        && seat.is_some_and(|s| scene_at(e.state(), e.content(), s.at));
    let quiet = (c.kind == Kind::Attends && !c.attend && !meets_scene)
        .then(|| quiet_target(e.state(), e.content()))
        .flatten();
    if let Some(target) = quiet {
        let before = e.state().calendar.turn;
        let skipped = e
            .apply(Command::Skip {
                target,
                limit: None,
            })
            .is_ok();
        let ran = e.state().calendar.turn - before;
        if skipped && ran > 0 {
            let stop = e
                .state()
                .act
                .last_skip
                .as_ref()
                .map(|s| stop_key(&s.stop))
                .unwrap_or_else(|| "open".into());
            c.skipped += ran;
            c.skips.push((ran, stop));
            // Whatever stopped it is met by a played turn.
            c.attend = true;
            return Stepped::Skipped(ran);
        }
    }
    c.attend = false;
    if let Some(seat) = seat {
        c.commands += play_turn_styled(e, c.kind, c.play, seat);
    }
    if e.apply(Command::EndTurn).is_err() {
        return Stepped::Over;
    }
    c.played += 1;
    Stepped::Played
}

// --- A start's rating (DEC-102 clause 6) ------------------------------------------------

/// Give a fork another future: every draw keyed by the world seed, and
/// every named stream, is re-keyed by `luck` (0: the game's own luck,
/// nothing changes). For a fork that is measured and dropped, never a
/// game that is played: a rating asks what the same present comes to
/// under other luck. The world as it stands is not touched; what is
/// re-derived from the seed on demand (which night a body meets, the
/// passers-by) comes out differently in the fork.
pub fn reluck(state: &mut State, luck: u64) {
    if luck == 0 {
        return;
    }
    let seed =
        mandate_core::rng::keyed(state.meta.seed, Stream::Misc, &[KEY_LUCK, luck]).next_u64();
    state.meta.seed = seed;
    state.rng = mandate_core::rng::RngStreams::new(seed);
}

/// One factor of the vote model's decomposition, as a start's rating
/// reports it: a fact and its Fluent key, never prose (DEC-054).
#[derive(Clone, Debug, PartialEq)]
pub struct Fact {
    /// `issues`, `party`, `valence`, `record`, `campaign`, `shock`.
    pub factor: String,
    /// `factor-<factor>`.
    pub key: String,
    /// The mean of the factor's share of the margin between the player
    /// and the other of the first two, in share points (0.01: one point),
    /// over the plays the player finished first or second in: above zero
    /// it worked for the player.
    pub margin: f64,
}

/// One play of a race.
#[derive(Clone, Debug, PartialEq)]
pub struct RacePlayed {
    pub luck: u64,
    /// Whether the player was on the ballot (not: the party passed them
    /// over, or the career ended first).
    pub on_ballot: bool,
    pub won: bool,
    pub share: f64,
    /// The decomposition, signed for the player (empty when they were not
    /// among the first two).
    pub factors: Vec<(String, f64)>,
    pub state_hash: String,
}

/// A start's rating: a race played under several lucks.
#[derive(Clone, Debug, PartialEq)]
pub struct Rating {
    pub post: Post,
    pub runs: u32,
    pub wins: u32,
    pub on_ballot: u32,
    pub win_rate: f64,
    pub mean_share: f64,
    /// The two or three factors that drive it: the largest by the size of
    /// their mean margin, largest first.
    pub facts: Vec<Fact>,
    pub plays: Vec<RacePlayed>,
}

/// How many facts a rating names.
pub const FACTS: usize = 3;

/// Play the player's next race for `post` (`None`: the post their next
/// election is for) once per luck, each on a fork of `state`, and rate it.
/// `turn` plays one turn's commands on the fork (the turn is ended here).
/// A pure function of the state, the content, the post and the lucks.
pub fn rate(
    state: &State,
    content: &Arc<ContentBundle>,
    post: Option<Post>,
    lucks: &[u64],
    turn: &mut dyn FnMut(&mut Engine),
) -> Result<Rating, String> {
    let c = content
        .constitutions
        .get(&state.meta.constitution)
        .ok_or("no constitution")?;
    let me = state.player.as_ref().ok_or("no player")?.politician;
    let held = crate::player::posts(state);
    let post = post
        .or_else(|| crate::skip::election_post(state, content))
        .ok_or("no post to run for")?;
    let (cal, term) =
        crate::politics::elections::calendar_for(c, post).ok_or("the post is not elected")?;
    let start = state.calendar.date;
    // The race in progress, if the player is already in it; else the next.
    let day = state
        .politics
        .elections
        .races
        .iter()
        .find(|r| r.seat == post)
        .map(|r| r.election_day)
        .unwrap_or_else(|| crate::politics::elections::next_election_day(cal, term, start));
    let mut plays = Vec::new();
    for &luck in lucks {
        let mut e =
            Engine::from_state(state.clone(), content.clone()).map_err(|x| x.to_string())?;
        e.setup(|st| reluck(st, luck));
        if !held.contains(&post) {
            // Filed where filing is done, as a player would.
            let file = e.route(PlayerCommand::File {
                filing: crate::politics::elections::Filing::Stand { post },
            });
            e.apply(file).map_err(|x| x.to_string())?;
        }
        // Turn by turn to the day's result (a turn is at most a week).
        let mut guard = start.days_until(day) + 16;
        while e.state().calendar.date <= day && guard > 0 {
            guard -= 1;
            if e.state().player.as_ref().is_some_and(|p| p.status.active()) {
                turn(&mut e);
            }
            if e.apply(Command::EndTurn).is_err() {
                break;
            }
        }
        let st = e.state();
        let result = st
            .politics
            .elections
            .results
            .iter()
            .find(|r| r.seat == post && r.date == day);
        let mine = result.and_then(|r| r.candidates.iter().find(|x| x.politician == me));
        let (won, share) = match (result, mine) {
            (Some(r), Some(m)) => (r.winner == me, m.share as f64),
            _ => (false, 0.0),
        };
        // The decomposition is the winner's margin over the runner-up.
        let factors = match (result, mine) {
            (Some(r), Some(_)) => {
                let mut order: Vec<&crate::politics::elections::CandidateResult> =
                    r.candidates.iter().collect();
                order.sort_by_key(|c| std::cmp::Reverse(c.votes));
                let second = order.get(1).is_some_and(|x| x.politician == me);
                if won || second {
                    let sign = if won { 1.0 } else { -1.0 };
                    r.decomposition
                        .iter()
                        .map(|f| (f.factor.clone(), sign * f.margin as f64))
                        .collect()
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        };
        plays.push(RacePlayed {
            luck,
            on_ballot: mine.is_some(),
            won,
            share,
            factors,
            state_hash: e.state_hash(),
        });
    }
    let runs = plays.len() as u32;
    let wins = plays.iter().filter(|p| p.won).count() as u32;
    let on_ballot = plays.iter().filter(|p| p.on_ballot).count() as u32;
    let mut sums: BTreeMap<String, (f64, u32)> = BTreeMap::new();
    for p in &plays {
        for (f, m) in &p.factors {
            let e = sums.entry(f.clone()).or_default();
            e.0 += m;
            e.1 += 1;
        }
    }
    let mut facts: Vec<Fact> = sums
        .into_iter()
        .map(|(factor, (sum, n))| Fact {
            key: format!("factor-{factor}"),
            factor,
            margin: sum / n.max(1) as f64,
        })
        .collect();
    facts.sort_by(|a, b| {
        b.margin
            .abs()
            .total_cmp(&a.margin.abs())
            .then(a.factor.cmp(&b.factor))
    });
    facts.truncate(FACTS);
    Ok(Rating {
        post,
        runs,
        wins,
        on_ballot,
        win_rate: wins as f64 / runs.max(1) as f64,
        mean_share: plays.iter().map(|p| p.share).sum::<f64>() / runs.max(1) as f64,
        facts,
        plays,
    })
}

/// [`rate`] with a stand-in playing the race.
pub fn rate_standin(
    state: &State,
    content: &Arc<ContentBundle>,
    post: Option<Post>,
    lucks: &[u64],
    kind: Kind,
) -> Result<Rating, String> {
    rate(state, content, post, lucks, &mut |e: &mut Engine| {
        if let Some(seat) = Seat::of_player(e.state(), e.content()) {
            play_turn(e, kind, seat);
        }
    })
}
