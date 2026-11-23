//! Bot tests: blind-invariance, knowledge and lobbying odds.

#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use mandate_sim::player::knowledge::Subject;

/// The members of the player's town's legislature.
fn legislature<'a>(
    st: &'a mandate_sim::State,
    content: &ContentBundle,
) -> &'a [mandate_sim::politics::PoliticianId] {
    let c = content.constitutions.get(&st.meta.constitution).unwrap();
    let town = mandate_sim::world::Jurisdiction::town(st.player.as_ref().unwrap().home());
    mandate_sim::politics::legislature_of(&st.politics, c, town)
}

fn content() -> Arc<ContentBundle> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    crate::lab::load(&root, &[]).unwrap_or_else(|e| panic!("{e}"))
}

/// An independent copy of a game (the same state, through the load path).
fn fork(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).expect("fork")
}

/// Perturb truths the player has not learned (M6_ENGINE_CONTRACTS
/// §1.10): the goals of people the player does not know in detail are
/// handed round among them, and two neighbourhoods whose worries the
/// player has not heard swap their residents' views (so their worries
/// swap too). Returns how many facts changed.
fn perturb(e: &mut Engine) -> usize {
    let mut changed = 0;
    let (unknown_people, hoods) = {
        let st = e.state();
        let k = Knowing::of(e);
        let pl = st.player.as_ref().expect("player");
        let unknown: Vec<CharacterId> = st
            .politics
            .characters
            .goals
            .iter()
            .map(|g| g.holder)
            .filter(|c| k.person(*c) < Level::Detailed)
            .collect();
        let ward = ward_hoods(&st.world, pl.ward);
        let hoods: Vec<usize> = st
            .world
            .hoods_of(pl.municipality)
            .filter(|h| !ward.contains(h))
            .filter(|h| {
                k.level(&Subject::HoodConcerns {
                    hood: NeighborhoodId(*h as u32),
                }) == Level::Unknown
            })
            .collect();
        (unknown, hoods)
    };
    e.setup(|s| {
        // Goals: each unknown holder's goals go to the next unknown holder.
        let holders: Vec<CharacterId> = {
            let mut v = unknown_people.clone();
            v.sort();
            v.dedup();
            v
        };
        if holders.len() >= 2 {
            for g in s.politics.characters.goals.iter_mut() {
                if let Some(i) = holders.iter().position(|h| *h == g.holder) {
                    g.holder = holders[(i + 1) % holders.len()];
                    changed += 1;
                }
            }
        }
        // Worries: two unheard neighbourhoods trade their residents' views.
        if let [a, b, ..] = hoods.as_slice() {
            let n = s.population.hoods[*a]
                .cohorts
                .len()
                .min(s.population.hoods[*b].cohorts.len());
            for i in 0..n {
                let x = s.population.hoods[*a].cohorts[i].attitudes.clone();
                let y = std::mem::replace(&mut s.population.hoods[*b].cohorts[i].attitudes, x);
                s.population.hoods[*a].cohorts[i].attitudes = y;
                changed += 1;
            }
        }
    });
    changed
}

/// The commands a bot sends in one turn from this game.
fn turn_commands(e: &Engine, content: &ContentBundle, strategy: &str, start: u32) -> Vec<String> {
    let mut g = fork(e);
    record(true);
    play_turn_from(&mut g, content, strategy, start);
    record(false)
}

/// Blind invariance (DEC-091; M6_ENGINE_CONTRACTS §1.10): a bot reads
/// gated facts only through `Knowing`, so perturbing truths it has not
/// learned leaves every command it sends unchanged, turn after turn, from
/// the start into the campaign.
#[test]
fn bots_are_blind_to_what_they_have_not_learned() {
    let content = content();
    for (strategy, seed) in [
        ("explorer", 3),
        ("mixed", 5),
        ("legislator", 2),
        ("corrupt", 4),
        ("canvass", 6),
        ("proxy", 7),
        ("explorer_campaigner", 8),
        ("explorer_coalition", 9),
        ("explorer_legislator", 10),
    ] {
        let mut e = new_game(&content, seed).expect("game");
        let start = e.state().calendar.turn;
        let mut perturbed = 0;
        let mut sent = 0;
        let mut campaign = false;
        for _ in 0..26 {
            campaign |= campaigning(&e);
            let plain = turn_commands(&e, &content, strategy, start);
            let mut p = fork(&e);
            perturbed += perturb(&mut p);
            let blind = turn_commands(&p, &content, strategy, start);
            assert_eq!(
                plain,
                blind,
                "{strategy} (seed {seed}, turn {}) acted on something it had not learned",
                e.state().calendar.turn
            );
            sent += plain.len();
            play_turn_from(&mut e, &content, strategy, start);
            if e.apply(Command::EndTurn).is_err() {
                break;
            }
        }
        assert!(perturbed > 0, "{strategy}: nothing was perturbed");
        assert!(sent > 0, "{strategy}: no commands were sent");
        assert!(campaign, "{strategy}: the campaign was never reached");
    }
}

/// The perturbation is real: the swapped neighbourhoods' worries change,
/// while the player's view of them does not.
#[test]
fn the_perturbation_changes_hidden_worries() {
    let content = content();
    let e = new_game(&content, 3).expect("game");
    let mut p = fork(&e);
    assert!(perturb(&mut p) > 0);
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let ward = ward_hoods(&st.world, pl.ward);
    let mut differs = false;
    for h in st
        .world
        .hoods_of(pl.municipality)
        .filter(|h| !ward.contains(h))
    {
        let hood = NeighborhoodId(h as u32);
        assert_eq!(
            Knowing::of(&e).concerns_of(hood),
            Knowing::of(&p).concerns_of(hood)
        );
        let truth = |g: &Engine| {
            mandate_sim::concerns::felt_concerns(
                &g.state().population,
                &content,
                pl.municipality.index(),
                &[h],
            )
        };
        differs |= truth(&e) != truth(&p);
    }
    assert!(differs, "no hidden worry changed");
}

/// The canvassing match shares one function with the command, and a
/// neighbourhood's real top worry beats one it barely feels.
#[test]
fn canvassing_on_the_real_worry_works_better() {
    let content = content();
    let e = new_game(&content, 1).expect("game");
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let hood = ward_hoods(&st.world, pl.ward)[0];
    let felt = mandate_sim::concerns::felt_concerns(
        &st.population,
        &content,
        pl.municipality.index(),
        &[hood],
    );
    let id = |i: usize| content.concerns[i].id.clone();
    let (top, last) = (id(felt[0].0), id(felt[felt.len() - 1].0));
    let m = |issue: Option<&str>| {
        mandate_sim::actions::canvass_match(st, &content, NeighborhoodId(hood as u32), issue)
    };
    let k = &content.knowledge.effects.canvass;
    assert_eq!(m(None), 1.0);
    assert!(m(Some(&top)) > 1.0 && m(Some(&top)) <= k.ceiling);
    assert!(m(Some(&last)) < 1.0 && m(Some(&last)) >= k.floor);
    // The command applies the same multiplier: a canvass on the top
    // worry raises approval more than one on the least felt.
    let gain = |issue: &str| {
        let mut g = fork(&e);
        let approval = |g: &Engine| -> f64 {
            g.state().population.hoods[hood]
                .cohorts
                .iter()
                .map(|c| c.player_approval as f64)
                .sum()
        };
        let before = approval(&g);
        let cmd = g.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: hood as u32,
                issue: Some(issue.to_string()),
            },
            late_night: false,
        });
        let r = g.apply(cmd).expect("canvass");
        (approval(&g) - before, r.result.unwrap().args)
    };
    let (hi, args) = gain(&top);
    let (lo, _) = gain(&last);
    assert!(hi > lo, "{hi} vs {lo}");
    assert_eq!(args.get("issue"), Some(&top));
    // An unknown worry is refused.
    let mut g = fork(&e);
    let cmd = g.route(PlayerCommand::Act {
        action: Action::Canvass {
            neighborhood: hood as u32,
            issue: Some("concern.nonsense".into()),
        },
        late_night: false,
    });
    assert!(g.apply(cmd).is_err());
}

/// The canvass curve in a real game (DEC-097 §1; ENGAGEMENT §2.1): the
/// campaign multiplier applies only while the player is a candidate in
/// an open race, and the command applies the curve the preview shows (a
/// repeat visit wins fewer over than a first one).
#[test]
fn the_campaign_multiplier_applies_only_while_a_candidate() {
    let content = content();
    let fx = &content.actions.canvass;
    let mut e = new_game(&content, 4).expect("game");
    let start = e.state().calendar.turn;
    let hood = |e: &Engine| {
        let st = e.state();
        ward_hoods(&st.world, st.player.as_ref().unwrap().ward)[0] as u32
    };
    let curve = |e: &Engine| {
        mandate_sim::player::ground::canvass_curve(e.state(), fx, NeighborhoodId(hood(e)))
    };
    assert!(!campaigning(&e));
    assert_eq!(curve(&e).season, 1.0);
    // Governing: a first visit, then a repeat, through the command.
    let gain = |e: &mut Engine| {
        let h = hood(e) as usize;
        let approval = |e: &Engine| -> f64 {
            e.state().population.hoods[h]
                .cohorts
                .iter()
                .map(|c| c.player_approval as f64)
                .sum()
        };
        let before = approval(e);
        let preview = curve(e);
        let cmd = e.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: h as u32,
                issue: None,
            },
            late_night: false,
        });
        let r = e.apply(cmd).expect("canvass");
        let band = r.result.unwrap().args.get("fresh").cloned();
        (approval(e) - before, preview, band)
    };
    let (first, c1, b1) = gain(&mut e);
    assert_eq!(c1.fresh, 1.0);
    assert_eq!(b1.as_deref(), Some("new"));
    e.apply(Command::EndTurn).expect("turn");
    let (second, c2, _) = gain(&mut e);
    assert!(c2.fresh < 0.75 && c2.season == 1.0, "{c2:?}");
    assert!(second < first, "{second} vs {first}");
    // Play on to the campaign: the multiplier holds while a candidate.
    let mut seen = false;
    for _ in 0..80 {
        if campaigning(&e) {
            seen = true;
            let c = curve(&e);
            assert_eq!(c.season, fx.campaign_mult);
            assert_eq!(c.approval, c.fresh * c.season * c.same_turn);
            break;
        }
        assert_eq!(curve(&e).season, 1.0);
        play_turn_from(&mut e, &content, "mixed", start);
        e.apply(Command::EndTurn).expect("turn");
    }
    assert!(seen, "the campaign was never reached");
    // After the election, back to governing.
    while campaigning(&e) {
        e.apply(Command::EndTurn).expect("turn");
    }
    assert_eq!(curve(&e).season, 1.0);
}

/// The lobbying odds the player is shown carry the knowledge term (the
/// shown and the rolled odds are one function), and knowing someone
/// better never makes them harder to persuade.
#[test]
fn lobbying_odds_follow_what_the_player_knows() {
    let content = content();
    let e = new_game(&content, 2).expect("game");
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let target = *legislature(st, &content)
        .iter()
        .find(|id| **id != pl.politician)
        .unwrap();
    let person = st.politics.get(target).unwrap().character;
    let odds = |level: Level| {
        let mut g = fork(&e);
        g.setup(|s| {
            let on = s.calendar.date;
            let k = &mut s.player.as_mut().unwrap().knowledge;
            k.entries.remove(&Subject::Person { person });
            if level > Level::Unknown {
                k.learn(
                    Subject::Person { person },
                    level,
                    on,
                    mandate_sim::player::knowledge::Source::Lab,
                );
            }
        });
        let shown = mandate_sim::actions::lobby_odds(g.state(), &content, target, false);
        let rolled = npc::lobby_odds_between(g.state(), &content, pl.politician, target, false);
        assert_eq!(shown, rolled);
        shown
    };
    let v: Vec<f64> = Level::ALL.iter().map(|l| odds(*l)).collect();
    assert!(v.windows(2).all(|w| w[0] <= w[1]), "{v:?}");
    assert!(v[3] > v[0], "{v:?}");
    // NPC deals use `known` (NPCs keep no ledger).
    let other = *legislature(st, &content)
        .iter()
        .find(|id| **id != pl.politician && **id != target)
        .unwrap();
    assert_eq!(
        npc::lobby_knowledge_logit(st, &content, other, target),
        content.knowledge.effects.lobby_logit.known
    );
}
