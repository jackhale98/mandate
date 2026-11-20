//! The player-proxy (DEC-012): it plays the player's turns during a time
//! skip by their standing directives.
//!
//! - **The same commands as a player.** It only proposes `PlayerCommand`s
//!   (routed like the UI's agenda, DEC-079), and the engine applies them as
//!   it would a player's, so outcomes are earned the same way.
//! - **Only what the player knows** (DEC-091): gated facts (worries, ward
//!   support) come through `Knowing`; the rest is public (the party's line,
//!   the bills on the floor, the player's own tracks and convictions).
//! - **Deterministic and keyed**: its draws are keyed by (seed,
//!   `Stream::Player`, `KEY_PROXY`, date, pick), never a sequential stream.
//! - **Modest** (DEC-097 balance principle): it never explores, lobbies or
//!   talks; it rotates the ground game around the ward rather than reading
//!   it; it leaves a share of its time unused (`proxy.idle_share`); it
//!   leaves climaxes and anything about a story or a case to the player.
//!   Skipping is a convenience, not the best way to play (`bots`' `directed`
//!   probe checks it stays below the informed styles).
//!
//! Use: make a `ProxyTurn` at the start of a turn; ask `next` for a command,
//! apply it, report with `result`; stop at `None`.

use crate::actions::{Action, PlayerCommand};
use crate::player::knowledge::{Knowing, PolicyRef};
use crate::skip::Settings;
use mandate_content::act::VoteStance;
use mandate_content::{ContentBundle, EffectDef, VoteChoice};
use mandate_core::Stream;

/// Keyed-RNG tag of the proxy's draws (DEC-041).
pub const KEY_PROXY: u64 = 0x9_0A11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lever {
    Ground,
    Fundraising,
    Press,
    Legislation,
}

#[derive(Clone, Debug, PartialEq)]
enum Pending {
    Answer { decision: u32, option: String },
    Vote { policy: u32 },
    Rest,
    Ad,
    Lever(Lever),
    Draft,
    Introduce,
}

/// One turn of the proxy.
#[derive(Clone, Debug)]
pub struct ProxyTurn {
    settings: Settings,
    last: Option<Pending>,
    tried: Vec<(u32, String)>,
    skipped: Vec<u32>,
    voted: Vec<u32>,
    rested: bool,
    ads: u8,
    ads_done: bool,
    picks: u64,
    exhausted: Vec<Lever>,
    commands: u8,
    done: bool,
    /// A bill drafted this turn, to introduce next.
    drafted: Option<u32>,
}

impl ProxyTurn {
    pub fn new(state: &crate::State, content: &ContentBundle) -> Self {
        ProxyTurn {
            settings: crate::skip::settings(&state.act.directives, content),
            last: None,
            tried: Vec::new(),
            skipped: Vec::new(),
            voted: Vec::new(),
            rested: false,
            ads: 0,
            ads_done: false,
            picks: 0,
            exhausted: Vec::new(),
            commands: 0,
            done: false,
            drafted: None,
        }
    }

    /// The decision and option the last command answered, if it did.
    pub fn answered(&self) -> Option<(u32, &str)> {
        match &self.last {
            Some(Pending::Answer { decision, option }) => Some((*decision, option.as_str())),
            _ => None,
        }
    }

    /// The bill and vote the last command declared, if it did.
    pub fn voted(&self, cmd: &PlayerCommand) -> Option<(u32, &'static str)> {
        match (&self.last, cmd) {
            (Some(Pending::Vote { .. }), PlayerCommand::CastVote { policy, vote }) => Some((
                *policy,
                match vote {
                    crate::politics::Vote::Yes => "yes",
                    crate::politics::Vote::No => "no",
                    crate::politics::Vote::Abstain => "abstain",
                },
            )),
            _ => None,
        }
    }

    /// How the last command went (`created`: the id it made, a draft's).
    pub fn result(&mut self, ok: bool, created: Option<u32>) {
        match self.last.clone() {
            Some(Pending::Lever(l)) if !ok => self.exhausted.push(l),
            Some(Pending::Rest) => self.rested = true,
            Some(Pending::Ad) => {
                if ok {
                    self.ads += 1;
                } else {
                    self.ads_done = true;
                }
            }
            Some(Pending::Draft) => {
                if ok {
                    self.drafted = created;
                } else {
                    self.exhausted.push(Lever::Legislation);
                }
            }
            Some(Pending::Introduce) => {
                self.drafted = None;
                if !ok {
                    self.exhausted.push(Lever::Legislation);
                }
            }
            _ => {}
        }
    }

    /// The next command, or `None` when the turn's work is done.
    pub fn next(&mut self, st: &crate::State, content: &ContentBundle) -> Option<PlayerCommand> {
        if self.done || self.commands >= content.act_flow.proxy.max_commands {
            return None;
        }
        let cmd = self.choose(st, content);
        if cmd.is_none() {
            self.done = true;
        } else {
            self.commands += 1;
        }
        cmd.map(|c| crate::places::routed(st, content, c))
    }

    fn choose(&mut self, st: &crate::State, content: &ContentBundle) -> Option<PlayerCommand> {
        let pl = st.player.as_ref()?;
        // 1. Decisions the proxy may answer.
        if let Some(c) = self.answer(st, content) {
            return Some(c);
        }
        // 2. Declared votes on the bills before the player's body.
        if let Some(c) = self.vote(st, content) {
            return Some(c);
        }
        if !pl.status.active() {
            return None;
        }
        // 3. Rest when worn out (caution).
        if !self.rested && pl.tracks.health < self.settings.rest_below && pl.slots_left() > 0 {
            self.last = Some(Pending::Rest);
            return Some(act(Action::Rest));
        }
        // 4. A bill drafted this turn goes before the body.
        if let Some(id) = self.drafted {
            self.last = Some(Pending::Introduce);
            return Some(PlayerCommand::IntroducePolicy { policy: id });
        }
        // 5. Money, not time: ads while a candidate, by fundraising effort.
        let campaigning = st
            .politics
            .elections
            .races
            .iter()
            .any(|r| r.candidates.iter().any(|c| c.politician == pl.politician));
        if campaigning && !self.ads_done && self.ads < self.settings.fundraising.min(3) {
            self.last = Some(Pending::Ad);
            return Some(act(Action::Advertise));
        }
        // 6. The slots, by effort, with some left unused.
        while pl.slots_left() > 0 {
            let mut rng = mandate_core::rng::keyed(
                st.meta.seed,
                Stream::Player,
                &[KEY_PROXY, st.calendar.date.0 as u64, self.picks],
            );
            self.picks += 1;
            if rng.chance(content.act_flow.proxy.idle_share) {
                return None;
            }
            let s = &self.settings;
            let levers: Vec<(Lever, u8)> = [
                (Lever::Ground, s.ground),
                (Lever::Fundraising, s.fundraising),
                (Lever::Press, s.press),
                (Lever::Legislation, s.legislation),
            ]
            .into_iter()
            .filter(|(l, w)| *w > 0 && !self.exhausted.contains(l))
            .collect();
            let total: u64 = levers.iter().map(|(_, w)| *w as u64).sum();
            if total == 0 {
                return None;
            }
            let mut pick = rng.below(total);
            let mut lever = levers[0].0;
            for (l, w) in &levers {
                if pick < *w as u64 {
                    lever = *l;
                    break;
                }
                pick -= *w as u64;
            }
            if let Some(c) = self.lever(st, content, lever) {
                return Some(c);
            }
            self.exhausted.push(lever);
        }
        None
    }

    fn answer(&mut self, st: &crate::State, content: &ContentBundle) -> Option<PlayerCommand> {
        let pending: Vec<&crate::narrative::Decision> = st
            .narrative
            .pending()
            .filter(|d| !crate::skip::reserved(d, content) && !self.skipped.contains(&d.id.0))
            .collect();
        for d in pending {
            let Some(def) = content.dilemma(&d.template) else {
                self.skipped.push(d.id.0);
                continue;
            };
            let mut opts: Vec<(f64, &str)> = def
                .options
                .iter()
                .filter(|o| !self.tried.iter().any(|(x, y)| *x == d.id.0 && y == &o.id))
                .map(|o| (score(st, content, &self.settings, d, o), o.id.as_str()))
                .collect();
            opts.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(b.1)));
            match opts.first() {
                Some((_, id)) => {
                    self.tried.push((d.id.0, id.to_string()));
                    self.last = Some(Pending::Answer {
                        decision: d.id.0,
                        option: id.to_string(),
                    });
                    return Some(PlayerCommand::Answer {
                        decision: d.id.0,
                        option: id.to_string(),
                    });
                }
                None => self.skipped.push(d.id.0),
            }
        }
        None
    }

    fn vote(&mut self, st: &crate::State, content: &ContentBundle) -> Option<PlayerCommand> {
        let r = crate::scheduler::Reads::of_state(st, content);
        let v = crate::narrative::cast::View::new(&r, &st.politics);
        let pl = st.player.as_ref()?;
        let bill = st
            .policy
            .in_municipality(pl.home().0)
            .map(|p| p.id.0)
            .find(|id| !self.voted.contains(id) && v.can_vote(*id))?;
        self.voted.push(bill);
        let sign = stance_sign(st, content, &self.settings, bill);
        self.last = Some(Pending::Vote { policy: bill });
        Some(PlayerCommand::CastVote {
            policy: bill,
            vote: if sign > 0 {
                crate::politics::Vote::Yes
            } else if sign < 0 {
                crate::politics::Vote::No
            } else {
                crate::politics::Vote::Abstain
            },
        })
    }

    fn lever(
        &mut self,
        st: &crate::State,
        content: &ContentBundle,
        lever: Lever,
    ) -> Option<PlayerCommand> {
        let pl = st.player.as_ref()?;
        let own = own_bill(st);
        self.last = Some(Pending::Lever(lever));
        match lever {
            Lever::Ground => {
                // Round the ward, a neighbourhood a pick (it does not read
                // where the player is least known).
                let hoods = crate::policy::ward_hoods(&st.world, pl.ward);
                if hoods.is_empty() {
                    return None;
                }
                let h = hoods[(st.calendar.turn as usize + self.picks as usize) % hoods.len()];
                let issue = self.settings.focus.clone().or_else(|| {
                    Knowing::new(st, content).top_worry(crate::world::NeighborhoodId(h as u32))
                });
                Some(act(Action::Canvass {
                    neighborhood: h as u32,
                    issue,
                }))
            }
            Lever::Fundraising => Some(act(Action::Fundraise)),
            Lever::Press => Some(act(Action::Press { policy: own })),
            Lever::Legislation => match own {
                Some(bill) => {
                    let hoods = crate::policy::ward_hoods(&st.world, pl.ward);
                    let h = *hoods.get(st.calendar.turn as usize % hoods.len().max(1))?;
                    Some(act(Action::TownHall {
                        neighborhood: h as u32,
                        policy: Some(bill),
                    }))
                }
                None => {
                    let t = best_template(st, content)?;
                    self.last = Some(Pending::Draft);
                    Some(PlayerCommand::DraftPolicy {
                        template: t,
                        params: Default::default(),
                    })
                }
            },
        }
    }
}

fn act(action: Action) -> PlayerCommand {
    PlayerCommand::Act {
        action,
        late_night: false,
    }
}

/// The player's bill before the body, if any.
fn own_bill(st: &crate::State) -> Option<u32> {
    let me = st.player.as_ref()?.politician;
    st.policy
        .instances
        .iter()
        .rev()
        .find(|p| {
            p.sponsor == Some(me) && p.status == crate::policy::PolicyStatus::UnderConsideration
        })
        .map(|p| p.id.0)
}

/// The template on the player's menu with the best ward support the player
/// knows of, that the town has not already got.
fn best_template(st: &crate::State, content: &ContentBundle) -> Option<String> {
    let pl = st.player.as_ref()?;
    let k = Knowing::new(st, content);
    let home = pl.home().0;
    let taken: Vec<&str> = st
        .policy
        .in_municipality(home)
        .filter(|p| !matches!(p.status, crate::policy::PolicyStatus::Ended))
        .map(|p| p.template.as_str())
        .collect();
    content
        .policies
        .values()
        .filter(|t| {
            k.on_menu(t)
                && !taken.contains(&t.id.as_str())
                && crate::policy::unlocked(st, content, t, home)
        })
        .map(|t| {
            let s = k
                .ward_support_of(&PolicyRef::Template {
                    template: t.id.clone(),
                })
                .unwrap_or(0.0);
            (s, t.id.clone())
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .filter(|(s, _)| *s > 0.0)
        .map(|(_, id)| id)
}

/// How the directives would vote on a bill: +1, −1 or 0.
fn stance_sign(st: &crate::State, content: &ContentBundle, s: &Settings, bill: u32) -> i8 {
    let Some(p) = st.policy.instances.get(bill as usize) else {
        return 0;
    };
    let Some(t) = content.policies.get(&p.template) else {
        return 0;
    };
    let sign = |x: f64| {
        if x > 0.05 {
            1
        } else if x < -0.05 {
            -1
        } else {
            0
        }
    };
    match s.vote {
        VoteStance::WithParty => {
            let r = crate::scheduler::Reads::of_state(st, content);
            crate::narrative::cast::View::new(&r, &st.politics).party_line(bill)
        }
        VoteStance::Conviction => {
            let Some(me) = st
                .player
                .as_ref()
                .and_then(|pl| st.politics.get(pl.politician))
            else {
                return 0;
            };
            sign(crate::politics::alignment(&t.signature_weights, &me.attitudes))
        }
        VoteStance::Ward => sign(
            Knowing::new(st, content)
                .ward_support_of(&PolicyRef::Template {
                    template: p.template.clone(),
                })
                .unwrap_or(0.0),
        ),
    }
}

/// A plain, cautious taste for an option: approval and standing first,
/// shady options refused above the directives' risk, votes by the stance.
fn score(
    st: &crate::State,
    content: &ContentBundle,
    s: &Settings,
    d: &crate::narrative::Decision,
    o: &mandate_content::OptionDef,
) -> f64 {
    o.effects
        .iter()
        .map(|e| match e {
            EffectDef::Opinion { delta, .. } => 0.05 * delta,
            EffectDef::Approval { delta, .. } => 60.0 * delta,
            EffectDef::Awareness { delta, .. } => 20.0 * delta,
            EffectDef::Money { amount } => 0.0003 * amount,
            EffectDef::Health { delta } => 0.2 * delta,
            EffectDef::Slots { n } => -(*n as f64),
            EffectDef::Standing { delta } => 0.5 * delta,
            EffectDef::LegalJeopardy { delta } => -0.5 * delta,
            EffectDef::Record { delta } => 20.0 * delta,
            EffectDef::Draft { .. } => 0.3,
            EffectDef::Hidden { evidence, .. } => {
                if *evidence as f32 > s.max_risk {
                    -1000.0
                } else {
                    -10.0 * evidence
                }
            }
            EffectDef::Vote { role, vote } => {
                let Some(crate::narrative::Actor::Policy { id }) =
                    crate::narrative::actor_of(&d.cast, role)
                else {
                    return 0.0;
                };
                let want = stance_sign(st, content, s, id) as f64;
                let line = {
                    let r = crate::scheduler::Reads::of_state(st, content);
                    crate::narrative::cast::View::new(&r, &st.politics).party_line(id) as f64
                };
                let got = match vote {
                    VoteChoice::Yes => 1.0,
                    VoteChoice::No => -1.0,
                    VoteChoice::Abstain => 0.0,
                    VoteChoice::WithParty => line,
                    VoteChoice::WithWard => {
                        stance_sign(
                            st,
                            content,
                            &Settings {
                                vote: VoteStance::Ward,
                                ..s.clone()
                            },
                            id,
                        ) as f64
                    }
                };
                1.5 * want * got
            }
            _ => 0.0,
        })
        .sum()
}
