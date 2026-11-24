#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The causal spine on a generated world (S1): each held hazard's strike
//! closes its own hold; a bill is prompted by an open proposal only.

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::environment::{DeferredHazard, HazardKind};
use mandate_sim::events::{EventId, EventKind};
use mandate_sim::policy::PolicyInstanceId;
use mandate_sim::policy::families::{FamilyInstance, FamilyStatus, LapseReason};
use mandate_sim::scheduler::{EffectQueue, Outs, Reads};
use mandate_sim::spine::{Cause, Link, Process, Shock, Thread};
use mandate_sim::world::Jurisdiction;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

/// Two hazards held over one town at once, both due: each late strike ends
/// its own hold, and each disaster continues from its own late strike.
#[test]
fn two_holds_over_one_town_never_cross() {
    let c = content();
    let e = generate(&WorldConfig::new(83, WorldSize::Small), c.clone()).unwrap();
    let mut st = e.state().clone();
    let town = st.player.as_ref().unwrap().home().0;
    let date = st.calendar.date;
    let max = c.director.chance.hazard_max_defer_days;
    // No hold in force and no thinning: both are due and both strike.
    st.narrative.chance.hazard_hold = None;
    st.narrative.chance.hazard_keep = None;
    let drawn = mandate_core::SimDate(date.0 - max - 1);
    let hold = |st: &mut mandate_sim::State, kind: HazardKind| -> EventId {
        let id = st.events.record(
            drawn,
            EventKind::DirectorIntervention {
                lever: "hazard_held".into(),
                municipality: town,
            },
            [
                Cause::Exogenous(Shock::Hazard),
                Cause::Process(Process::Director),
            ],
        );
        st.environment.deferred.push(DeferredHazard {
            municipality: town,
            kind,
            severity: 0.5,
            drawn,
            hold_event: Some(id),
        });
        id
    };
    let first = hold(&mut st, HazardKind::Flood);
    let second = hold(&mut st, HazardKind::Storm);
    let before = st.events.iter().last().unwrap().id;
    // One step of the hazard system.
    let snapshot = st.clone();
    let r = Reads::of_state(&snapshot, &c);
    let mut rng = snapshot.rng.clone();
    let mut q = EffectQueue::default();
    let mut out = Outs {
        rng: &mut rng,
        events: &mut st.events,
        effects: &mut q,
    };
    mandate_sim::environment::hazards(&mut st.environment, &r, &mut out);
    assert!(st.environment.deferred.is_empty(), "both struck");
    let log = &st.events;
    let late: Vec<_> = log
        .after(before)
        .iter()
        .filter(|x| {
            matches!(&x.kind, EventKind::DirectorIntervention { lever, municipality }
                if lever == "hazard_struck_late" && *municipality == town)
        })
        .collect();
    assert_eq!(late.len(), 2);
    // Each late strike ends its own hold, and only that one.
    for (strike, own) in late.iter().zip([first, second]) {
        let links: Vec<_> = strike.links().collect();
        assert_eq!(links, vec![(own, Link::Ended)]);
    }
    // Each disaster continues from its own late strike.
    let disasters: Vec<_> = log
        .after(before)
        .iter()
        .filter(
            |x| matches!(x.kind, EventKind::Disaster { municipality, .. } if municipality == town),
        )
        .filter(|x| x.links().next().is_some())
        .collect();
    assert_eq!(disasters.len(), 2);
    for (d, strike) in disasters.iter().zip(&late) {
        assert_eq!(
            d.links().collect::<Vec<_>>(),
            vec![(strike.id, Link::Continues)]
        );
        assert!(d.causes.contains(&Cause::Exogenous(Shock::Hazard)));
    }
    // The floods and the storm are told apart all the way back.
    let hazard_of = |d: &mandate_sim::events::DomainEvent| match d.kind {
        EventKind::Disaster { hazard, .. } => hazard,
        _ => unreachable!(),
    };
    assert_eq!(hazard_of(disasters[0]), HazardKind::Flood);
    assert_eq!(hazard_of(disasters[1]), HazardKind::Storm);
}

/// A bill drafted for a proposal still on the table is prompted by it; one
/// drafted after the proposal lapsed is not (its thread's last event is the
/// lapse, which prompted nothing).
#[test]
fn only_an_open_proposal_prompts_a_bill() {
    let c = content();
    let e = generate(&WorldConfig::new(83, WorldSize::Small), c.clone()).unwrap();
    let mut st = e.state().clone();
    let date = st.calendar.date;
    let at = Jurisdiction::town(st.player.as_ref().unwrap().home());
    let bill = PolicyInstanceId(900_000);
    let id = st.policy.families.ids.alloc();
    let family: String = "family.test".into();
    st.policy.families.instances.push(FamilyInstance {
        id,
        family: family.clone(),
        at,
        bindings: Vec::new(),
        sponsors: Vec::new(),
        opened: date,
        trigger_held: date,
        status: FamilyStatus::Open,
        drafts: vec![bill],
        args: Default::default(),
    });
    let opened = st.events.record(
        date,
        EventKind::ProposalOpened {
            proposal: id.0,
            family: family.clone(),
            at,
        },
        Cause::Process(Process::Agenda),
    );
    assert_eq!(mandate_sim::spine::proposal_of(&st, bill), Some(opened));
    // Another bill belongs to no proposal.
    assert_eq!(
        mandate_sim::spine::proposal_of(&st, PolicyInstanceId(900_001)),
        None
    );
    // The proposal lapses.
    let lapsed = st.events.record(
        date,
        EventKind::ProposalLapsed {
            proposal: id.0,
            family,
            at,
        },
        Cause::Process(Process::Agenda),
    );
    if let Some(f) = st.policy.families.instances.last_mut() {
        f.status = FamilyStatus::Lapsed {
            on: date,
            reason: LapseReason::Eased,
        };
    }
    assert_eq!(mandate_sim::spine::proposal_of(&st, bill), None);
    // The thread still knows how it ended (for what is said of it later).
    assert_eq!(st.events.head(Thread::Proposal(id.0)), Some(lapsed));
    assert_eq!(st.events.open_head(Thread::Proposal(id.0)), None);
}
