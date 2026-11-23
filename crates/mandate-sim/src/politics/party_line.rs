//! The party line (DEC-027, DEC-030): where a party stands on a bill.
//!
//! One rule for the player's whip and for NPC members. A party's position
//! is its **platform against the bill's attitude axes** (the template's
//! `signature_weights`), adjusted by who sponsors the bill: the party's own
//! member, the governing party (the one holding the signing office), or
//! anyone else (a mild lean against). A score inside the free band is a
//! free vote. The result carries a reason: a Fluent key and the issue the
//! stance rests on (an attitude index, shown as `issue-<key>`).

use super::{PartyId, PoliticianId, PoliticsState, alignment};
use crate::policy::PolicyState;
use mandate_content::{ContentBundle, PartyLineParams};

/// Who sponsors a bill, as a party sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sponsor {
    Own,
    Governing,
    Other,
    /// No sponsor, or a sponsor with no party.
    Unknown,
}

/// Why the line is what it is: a Fluent key and the issue behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reason {
    pub key: &'static str,
    /// The attitude (registry index) the bill and the platform meet on.
    pub issue: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stance {
    /// Platform fit and sponsor term, summed (positive: for the bill).
    pub score: f64,
    /// +1 yes, −1 no, 0 free vote.
    pub line: i8,
    pub reason: Reason,
}

/// How a bill's sponsor relates to a party.
pub fn sponsor_relation(
    party: Option<PartyId>,
    sponsor: Option<PartyId>,
    governing: Option<PartyId>,
) -> Sponsor {
    match (party, sponsor) {
        (Some(a), Some(b)) if a == b => Sponsor::Own,
        (Some(_), Some(b)) if governing == Some(b) => Sponsor::Governing,
        (Some(_), Some(_)) => Sponsor::Other,
        _ => Sponsor::Unknown,
    }
}

/// The attitude where platform and bill weigh most (largest product).
fn main_issue(platform: &[f32], signature: &[f64]) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, (s, p)) in signature.iter().zip(platform).enumerate() {
        let w = (s * f64::from(*p)).abs();
        if w > 1e-9 && best.is_none_or(|(_, b)| w > b + 1e-12) {
            best = Some((i, w));
        }
    }
    best.map(|(i, _)| i)
}

/// A party's stance on a bill from its platform and the sponsor.
pub fn stance(
    k: &PartyLineParams,
    platform: &[f32],
    signature: &[f64],
    sponsor: Sponsor,
) -> Stance {
    let fit = alignment(signature, platform);
    let platform_term = k.platform * fit;
    let sponsor_term = match sponsor {
        Sponsor::Own => k.own_sponsor,
        Sponsor::Governing => k.governing_sponsor,
        Sponsor::Other => k.other_sponsor,
        Sponsor::Unknown => 0.0,
    };
    let score = platform_term + sponsor_term;
    let line = if score >= k.free_band {
        1
    } else if score <= -k.free_band {
        -1
    } else {
        0
    };
    let issue = main_issue(platform, signature);
    // The bigger of the two terms carries the reason.
    let by_platform = platform_term.abs() >= sponsor_term.abs();
    let key = if line == 0 {
        // Torn: the platform and the sponsor pull opposite ways.
        if platform_term * sponsor_term < 0.0 && platform_term.abs() > 0.05 {
            "party-reason-free-torn"
        } else {
            "party-reason-free"
        }
    } else if by_platform {
        if platform_term > 0.0 {
            "party-reason-fits"
        } else {
            "party-reason-cuts"
        }
    } else {
        match sponsor {
            Sponsor::Own => "party-reason-own",
            Sponsor::Governing => "party-reason-governing",
            _ => "party-reason-rival",
        }
    };
    let issue = if key.starts_with("party-reason-fits")
        || key.starts_with("party-reason-cuts")
        || key == "party-reason-free-torn"
    {
        issue
    } else {
        None
    };
    Stance {
        score,
        line,
        reason: Reason { key, issue },
    }
}

/// A member's stance on a bill: their party's, if they have one.
pub fn member_stance(
    k: &PartyLineParams,
    pol: &PoliticsState,
    party: Option<PartyId>,
    sponsor_party: Option<PartyId>,
    governing: Option<PartyId>,
    signature: &[f64],
) -> Option<Stance> {
    let platform = &pol.party(party?)?.platform;
    let rel = sponsor_relation(party, sponsor_party, governing);
    Some(stance(k, platform, signature, rel))
}

/// How hard an NPC member is pulled by their party (lean units, before the
/// member's loyalty): nothing in a free vote.
pub fn npc_pull(
    k: &PartyLineParams,
    pol: &PoliticsState,
    party: Option<PartyId>,
    sponsor_party: Option<PartyId>,
    governing: Option<PartyId>,
    signature: &[f64],
) -> f64 {
    match member_stance(k, pol, party, sponsor_party, governing, signature) {
        Some(s) if s.line != 0 => k.npc_pull * s.score,
        _ => 0.0,
    }
}

/// The player's party's stance on a bill (the whip). `None` without a party
/// or a bill.
pub fn player_stance(
    content: &ContentBundle,
    pol: &PoliticsState,
    policy: &PolicyState,
    constitution: &str,
    player: PoliticianId,
    bill: u32,
) -> Option<Stance> {
    let me = pol.get(player)?;
    let inst = policy.instances.get(bill as usize)?;
    let t = crate::policy::resolve(content, policy, inst)?;
    let sponsor_party = inst.sponsor.and_then(|s| pol.get(s)).and_then(|s| s.party);
    let governing = content
        .constitutions
        .get(constitution)
        .and_then(|c| super::executive_of(pol, c, inst.at()))
        .and_then(|e| pol.get(e))
        .and_then(|e| e.party);
    member_stance(
        &content.politics.party_line,
        pol,
        me.party,
        sponsor_party,
        governing,
        &t.signature_weights,
    )
}
