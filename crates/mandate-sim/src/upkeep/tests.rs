#![allow(clippy::unwrap_used)]
use super::*;

fn fault(id: u32, opened: u32, expedited: Option<u32>) -> Fault {
    Fault {
        id: FaultId(id),
        kind: "fault.x".into(),
        hood: NeighborhoodId(0),
        spot: 0,
        opened: SimDate(opened),
        status: FaultStatus::Open,
        expedited: expedited.map(SimDate),
        ahead_of: None,
        put_back_for: None,
    }
}

/// The queue: faults moved up first (by the day they were), then the
/// oldest; the natural order ignores moving up.
#[test]
fn moved_up_faults_go_first_then_the_oldest() {
    let mut q = [
        fault(0, 10, None),
        fault(1, 5, None),
        fault(2, 20, Some(30)),
        fault(3, 15, Some(25)),
    ];
    q.sort_by_key(|f| queue_key(f, true));
    let ids: Vec<u32> = q.iter().map(|f| f.id.0).collect();
    assert_eq!(ids, [3, 2, 1, 0]);
    q.sort_by_key(|f| queue_key(f, false));
    let ids: Vec<u32> = q.iter().map(|f| f.id.0).collect();
    assert_eq!(ids, [1, 0, 3, 2]);
}

/// Money pays the queue in order and stops at the first that does not fit
/// (no smaller one jumps it).
#[test]
fn the_fund_stops_at_the_first_fault_it_cannot_pay_for() {
    let q = [
        (FaultId(0), 500.0),
        (FaultId(1), 900.0),
        (FaultId(2), 100.0),
    ];
    assert_eq!(paid(&q, 1400.0), vec![FaultId(0), FaultId(1)]);
    assert_eq!(paid(&q, 1300.0), vec![FaultId(0)]);
    assert!(paid(&q, 0.0).is_empty());
    // A job dearer than the month's money takes the month at the head.
    assert_eq!(paid(&q[1..], 400.0), vec![FaultId(1)]);
}

/// An empty domain is never written, so a game with no fault content
/// hashes as it did before upkeep.
#[test]
fn an_empty_domain_is_not_written() {
    let u = UpkeepState::default();
    assert!(u.is_empty());
    let mut u = u;
    let _: FaultId = u.ids.alloc();
    assert!(!u.is_empty());
}
