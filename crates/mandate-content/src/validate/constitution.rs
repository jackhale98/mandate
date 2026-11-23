//! Constitutions (DEC-017, DEC-098): procedures, posts, careers and nomination gates.

use super::*;

/// A constitution's posts and career (DEC-017, DEC-098): ids and role words
/// are unique, every tier has the engine's procedures, only implemented
/// voting rules, at-large bodies have seats, workplaces are place kinds,
/// and the career ladder reaches its summit.
pub(super) fn validate_posts(
    b: &ContentBundle,
    c: &crate::types::ConstitutionDef,
    issues: &mut Vec<Issue>,
) {
    use crate::types::{Constituency, PostKey, PostRef, Selection, TierKey, VotingRule};
    let err = |issues: &mut Vec<Issue>, msg: String| issues.push(Issue::error(&c.id, &msg));
    let mut ids: Vec<&str> = Vec::new();
    let mut words: Vec<&str> = Vec::new();
    for t in &c.tiers {
        // The engine's procedures (a contract: law-making runs `ordinance`).
        for needed in crate::types::ENGINE_PROCEDURES {
            if t.procedure(needed).is_none() {
                err(
                    issues,
                    format!("{:?} tier lacks the `{needed}` procedure", t.tier),
                );
            }
        }
        for p in t.posts() {
            let id = p.id();
            if ids.contains(&id) {
                err(
                    issues,
                    format!("body or office id `{id}` is used twice (ids are unique across tiers)"),
                );
            }
            ids.push(id);
            if !valid_key(id) || id.contains('.') || PostKey::new(id).is_none() {
                err(
                    issues,
                    format!(
                        "body or office id `{id}` must be [a-z0-9_], at most {} bytes",
                        PostKey::MAX
                    ),
                );
            }
            let word = p.word();
            if words.contains(&word) {
                err(issues, format!("role word `{word}` names two posts"));
            }
            words.push(word);
            if !valid_key(word) || word.contains('.') {
                err(issues, format!("role word `{word}` must be [a-z0-9_]"));
            }
            if b.occupation(word).is_some()
                || word == crate::narrative::OFFICE_ANY
                || word == crate::narrative::OFFICE_NONE
            {
                err(
                    issues,
                    format!("role word `{word}` is already an occupation or a keyword"),
                );
            }
            if p.term_years() == 0 {
                err(issues, format!("`{id}` needs a term of at least a year"));
            }
            if !b.places.iter().any(|x| x.key == p.works_at()) {
                err(
                    issues,
                    format!(
                        "`{id}` works at `{}`, which is not a place kind",
                        p.works_at()
                    ),
                );
            }
            match p.selection() {
                Selection::Election { rule, constituency } => {
                    // Only the plurality tabulator exists (DEC-098 note).
                    if *rule != VotingRule::Plurality {
                        err(
                            issues,
                            format!(
                                "`{id}` uses the voting rule {rule:?}, which has no tabulator yet (only plurality)"
                            ),
                        );
                    }
                    match (p, constituency) {
                        (PostRef::Body(body), Constituency::AtLarge) => {
                            if body.seats.unwrap_or(0) == 0 {
                                err(issues, format!("at-large body `{id}` needs `seats`"));
                            }
                            if t.tier == TierKey::Local {
                                // M6 instantiates local posts only, and has
                                // no at-large races for bodies yet.
                                err(
                                    issues,
                                    format!(
                                        "at-large body `{id}` is not supported on the local tier yet"
                                    ),
                                );
                            }
                        }
                        (PostRef::Body(body), Constituency::Ward) => {
                            if body.seats.is_some() {
                                err(
                                    issues,
                                    format!(
                                        "`{id}` has a seat per ward: `seats` is for at-large bodies"
                                    ),
                                );
                            }
                        }
                        (PostRef::Office(_), Constituency::Ward) => {
                            err(issues, format!("office `{id}` cannot be elected by ward"));
                        }
                        (PostRef::Office(_), Constituency::AtLarge) => {}
                    }
                }
                Selection::AppointedBy { office } => {
                    if t.office(office).is_none() {
                        err(
                            issues,
                            format!("`{id}` is appointed by unknown office `{office}`"),
                        );
                    }
                }
                Selection::ElectedByBody { body } => {
                    if t.body(body).is_none() {
                        err(
                            issues,
                            format!("`{id}` is elected by unknown body `{body}`"),
                        );
                    }
                }
            }
            if let Some(cal) = p.calendar()
                && !(1..=12).contains(&cal.month)
            {
                err(issues, format!("`{id}` has an election month outside 1–12"));
            }
        }
    }
    // Role words are distinct from ids of other posts (an office's id is its
    // own word).
    for t in &c.tiers {
        for body in &t.bodies {
            if c.post(&body.member_key)
                .is_some_and(|(_, p)| p.id() != body.id)
            {
                err(
                    issues,
                    format!("`{}`'s member word is another post's id", body.id),
                );
            }
        }
    }
    // The career ladder (DEC-017: a viable path to the summit).
    let ladder = &c.career;
    let Some((summit_tier, summit)) = c.post(&ladder.summit) else {
        err(
            issues,
            format!("career summit `{}` is not a post", ladder.summit),
        );
        return;
    };
    let reachable = |p: PostRef<'_>| {
        matches!(
            p.selection(),
            Selection::Election { .. } | Selection::ElectedByBody { .. }
        )
    };
    if !reachable(summit) {
        err(issues, "the career summit must be elected".into());
    }
    let mut tiers: Vec<TierKey> = c.tiers.iter().map(|t| t.tier).collect();
    tiers.sort();
    for tk in tiers.iter().filter(|t| **t < summit_tier) {
        if !c.tier(*tk).is_some_and(|t| t.posts().any(reachable)) {
            err(
                issues,
                format!("no elected post on the {tk:?} tier: no path to the summit"),
            );
        }
    }
    if ladder.acts.is_empty() {
        err(issues, "the career needs acts".into());
    }
    let mut act_ids: Vec<&str> = Vec::new();
    for a in &ladder.acts {
        if act_ids.contains(&a.id.as_str()) {
            err(issues, format!("act `{}` defined twice", a.id));
        }
        act_ids.push(&a.id);
        if a.tier.is_none() && a.posts.is_empty() {
            err(issues, format!("act `{}` needs a tier or posts", a.id));
        }
        if a.tier.is_some_and(|tk| c.tier(tk).is_none()) {
            err(
                issues,
                format!("act `{}` names a tier the constitution lacks", a.id),
            );
        }
        for p in a.posts.iter().chain(&a.except) {
            if c.post(p).is_none() {
                err(issues, format!("act `{}` names unknown post `{p}`", a.id));
            }
        }
    }
    for t in &c.tiers {
        for p in t.posts() {
            if c.act_of(t.tier, p.id()).is_none() {
                err(issues, format!("post `{}` belongs to no act", p.id()));
            }
        }
    }
}

/// Constitutions: procedure stages, posts, and a playable local tier.
pub(super) fn validate_constitutions(b: &ContentBundle, issues: &mut Vec<Issue>) {
    // Constitutions: every procedure stage references a real body or office,
    // and the local tier can propose and approve ordinances (playability).
    for c in b.constitutions.values() {
        let Some(local) = c.tier(crate::types::TierKey::Local) else {
            issues.push(Issue::error(&c.id, "constitution has no local tier"));
            continue;
        };
        for t in &c.tiers {
            for p in &t.procedures {
                if p.stages.is_empty() {
                    issues.push(Issue::error(
                        &c.id,
                        &format!("procedure `{}` has no stages", p.id),
                    ));
                }
                for st in &p.stages {
                    match st {
                        crate::types::StageDef::Vote { body, .. } => {
                            if t.body(body).is_none() {
                                issues.push(Issue::error(
                                    &c.id,
                                    &format!("procedure `{}` votes in unknown body `{body}`", p.id),
                                ));
                            }
                        }
                        crate::types::StageDef::Sign {
                            office,
                            override_body,
                            ..
                        } => {
                            if t.office(office).is_none() {
                                issues.push(Issue::error(
                                    &c.id,
                                    &format!(
                                        "procedure `{}` needs unknown office `{office}`",
                                        p.id
                                    ),
                                ));
                            }
                            if let Some(ob) = override_body
                                && t.body(ob).is_none()
                            {
                                issues.push(Issue::error(
                                    &c.id,
                                    &format!("override body `{ob}` unknown"),
                                ));
                            }
                        }
                    }
                }
            }
        }
        validate_posts(b, c, issues);
        if local
            .holder_of(crate::types::Power::ProposeOrdinance)
            .is_none()
        {
            issues.push(Issue::error(
                &c.id,
                "no local holder of propose_ordinance (unplayable)",
            ));
        }
    }
    if b.constitutions.is_empty() {
        issues.push(Issue::error("<content>", "no constitution defined"));
    }
}

/// Nomination gates name posts by role word (DEC-098).
pub(super) fn validate_nomination(b: &ContentBundle, issues: &mut Vec<Issue>) {
    // Nomination gates name posts by role word (DEC-098).
    for (word, gate) in &b.career.nomination {
        if !b.is_role_word(word) {
            issues.push(Issue::error(
                "career",
                &format!("nomination gate for unknown role word `{word}`"),
            ));
        }
        if gate
            .min_standing
            .is_some_and(|m| !(m.is_finite() && (0.0..=100.0).contains(&m)))
        {
            issues.push(Issue::error(
                "career",
                &format!("nomination gate for `{word}`: min_standing must be in 0–100"),
            ));
        }
    }
}
