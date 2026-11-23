//! The party line (owner note 8): a party's position on a bill comes from
//! its platform against the bill's attitude axes, adjusted by who sponsors
//! it; a band around zero is a free vote; the result says why.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_sim::politics::party_line::{Sponsor, stance};

fn content() -> mandate_content::ContentBundle {
    mandate_content::load(&mandate_content::embedded::sources())
        .unwrap()
        .0
}

/// A platform from (attitude key, value) pairs, in registry order.
fn platform(c: &mandate_content::ContentBundle, v: &[(&str, f32)]) -> Vec<f32> {
    c.attitudes
        .iter()
        .map(|a| v.iter().find(|(k, _)| *k == a.key).map_or(0.0, |x| x.1))
        .collect()
}

fn signature(c: &mandate_content::ContentBundle, template: &str) -> Vec<f64> {
    c.policies.get(template).unwrap().signature_weights.clone()
}

#[test]
fn a_liberal_reform_party_does_not_whip_against_housing_because_a_rival_proposed_it() {
    let c = content();
    let k = &c.politics.party_line;
    let liberal = platform(
        &c,
        &[
            ("social_liberalism", 0.8),
            ("reform_appetite", 0.7),
            ("redistribution", 0.4),
        ],
    );
    let housing = signature(&c, "policy.affordable_housing");
    let s = stance(k, &liberal, &housing, Sponsor::Other);
    assert!(s.line >= 0, "not whipped against: {s:?}");
    assert_eq!(s.line, 1, "it fits the platform: {s:?}");
    assert_eq!(s.reason.key, "party-reason-fits");
    assert!(s.reason.issue.is_some());
    // The same bill from a rival is a mild lean against, not a veto: a party
    // with no stance on it is left free.
    let blank = platform(&c, &[]);
    let s = stance(k, &blank, &housing, Sponsor::Other);
    assert_eq!(s.line, 0, "a mild lean stays inside the free band: {s:?}");
    assert_eq!(s.reason.key, "party-reason-free");
}

#[test]
fn an_own_bill_against_the_platform_is_a_free_vote_or_against() {
    let c = content();
    let k = &c.politics.party_line;
    let housing = signature(&c, "policy.affordable_housing");
    // A party opposed to everything the bill stands for.
    let against: Vec<f32> = housing.iter().map(|w| -w.signum() as f32).collect();
    let strong = stance(k, &against, &housing, Sponsor::Own);
    assert_eq!(
        strong.line, -1,
        "a strong contradiction is against: {strong:?}"
    );
    assert_eq!(strong.reason.key, "party-reason-cuts");
    // A mild one is a free vote, noting the pull of both.
    let mild: Vec<f32> = housing.iter().map(|w| -w.signum() as f32 * 0.25).collect();
    let free = stance(k, &mild, &housing, Sponsor::Own);
    assert_eq!(free.line, 0, "{free:?}");
    assert_eq!(free.reason.key, "party-reason-free-torn");
    // With no stance at all, a member's own bill is backed, as their own.
    let blank = platform(&c, &[]);
    let own = stance(k, &blank, &housing, Sponsor::Own);
    assert_eq!((own.line, own.reason.key), (1, "party-reason-own"));
}
