//! Static checks: references to ids that do not exist, reachability, roles and voices.

use super::*;

pub(super) fn static_checks(b: &ContentBundle, out: &mut Out) {
    let concern = |k: &str| b.concerns.iter().any(|c| c.id == k);
    let roles_all = b.talk_roles();
    let unknown = |out: &mut Out, item: &str, what: &str, id: &str| {
        out.add(
            Sev::Error,
            "unknown-ref",
            item,
            format!("{what} `{id}` does not exist"),
        );
    };
    // Dilemmas.
    for d in &b.dilemmas {
        let mut roles: Vec<&str> = d.roles.iter().map(|r| r.id.as_str()).collect();
        // A storyline stage's dilemma may name the storyline's roles.
        for s in &b.storylines {
            if s.stages
                .iter()
                .any(|x| x.decision.as_deref() == Some(d.id.as_str()))
            {
                roles.extend(s.roles.iter().map(|r| r.id.as_str()));
            }
        }
        check_conditions(out, &d.id, "triggers", &d.triggers);
        cond_refs(b, out, &d.id, Some(&roles), &d.triggers);
        if let Some(s) = &d.scene
            && b.scene(s).is_none()
        {
            unknown(out, &d.id, "scene", s);
        }
        if d.category == mandate_content::DilemmaCategory::Request
            && !b
                .goals
                .iter()
                .any(|g| g.request.as_deref() == Some(d.id.as_str()))
        {
            out.add(
                Sev::Error,
                "unsatisfiable",
                &d.id,
                "a request that no goal raises can never come up",
            );
        }
        if d.storyline_only
            && !b.storylines.iter().any(|s| {
                s.stages
                    .iter()
                    .any(|x| x.decision.as_deref() == Some(d.id.as_str()))
            })
            // An arc's decision beat presents it (DEC-109).
            && !b.arcs.iter().any(|a| {
                a.beats().any(|(_, x)| {
                    matches!(&x.kind, mandate_content::arcs::BeatKind::Decision { dilemma, .. }
                        if dilemma == &d.id)
                })
            })
        {
            out.add(
                Sev::Error,
                "unsatisfiable",
                &d.id,
                "storyline-only, but no storyline stage or arc beat presents it",
            );
        }
        for r in &d.roles {
            check_cast(b, out, &d.id, &r.cast);
        }
    }
    // Goals.
    for g in &b.goals {
        check_conditions(out, &g.id, "conditions", &g.conditions);
        cond_refs(b, out, &g.id, Some(&["holder"]), &g.conditions);
        for (axis, [lo, hi]) in &g.personality {
            if lo > hi || *lo > 1.0 || *hi < 0.0 {
                out.add(
                    Sev::Error,
                    "unsatisfiable",
                    &g.id,
                    format!("holder's {axis} in [{lo}, {hi}] never holds"),
                );
            }
        }
        if let Some(r) = &g.request
            && b.dilemma(r).is_none()
        {
            unknown(out, &g.id, "request dilemma", r);
        }
    }
    // Storylines.
    for s in &b.storylines {
        let roles: Vec<&str> = s.roles.iter().map(|r| r.id.as_str()).collect();
        check_conditions(out, &s.id, "start", &s.start);
        cond_refs(b, out, &s.id, Some(&roles), &s.start);
        for st in &s.stages {
            check_conditions(out, &s.id, &format!("stage {} advance", st.id), &st.advance);
            cond_refs(b, out, &s.id, Some(&roles), &st.advance);
            if let Some(d) = &st.decision
                && b.dilemma(d).is_none()
            {
                unknown(out, &s.id, "stage decision", d);
            }
        }
        if let Some(f) = &s.follow_up
            && b.storyline(f).is_none()
        {
            unknown(out, &s.id, "follow-up storyline", f);
        }
        for r in &s.roles {
            check_cast(b, out, &s.id, &r.cast);
        }
    }
    // Scenes nobody presents.
    for sc in &b.scenes {
        if !b
            .dilemmas
            .iter()
            .any(|d| d.scene.as_deref() == Some(sc.id.as_str()))
        {
            out.add(
                Sev::Error,
                "unsatisfiable",
                &sc.id,
                "no dilemma presents this scene",
            );
        }
    }
    // Situational policies: each alternative of the unlock.
    for p in b.policies.values() {
        if let Some(u) = &p.unlock {
            for (i, c) in u.any.iter().enumerate() {
                check_conditions(
                    out,
                    &p.id,
                    &format!("unlock alternative {}", i + 1),
                    std::slice::from_ref(c),
                );
                cond_refs(b, out, &p.id, None, std::slice::from_ref(c));
            }
        }
    }
    // Overheard lines.
    for o in &b.overheard {
        check_conditions(out, &o.id, "when", &o.when);
        cond_refs(b, out, &o.id, None, &o.when);
        if let Some(c) = &o.concern
            && !concern(c)
        {
            unknown(out, &o.id, "concern", c);
        }
    }
    // Topics, voices, lines; with the voice and topic they need.
    for t in &b.talk_topics {
        check_conditions(out, &t.id, "offered when", &t.offered.when);
        cond_refs(b, out, &t.id, None, &t.offered.when);
        for r in &t.offered.roles {
            if !roles_all.contains(r) {
                unknown(out, &t.id, "role", r);
            }
        }
    }
    // A voice never wins below an unconditional voice of higher priority
    // (or of equal priority, loaded earlier: ties go to load order).
    for (vi, v) in b.talk_voices.iter().enumerate() {
        check_conditions(out, &v.id, "when", &v.when);
        cond_refs(b, out, &v.id, None, &v.when);
        if let Some(w) = b.talk_voices.iter().enumerate().find_map(|(wi, w)| {
            (wi != vi
                && w.when.is_empty()
                && (w.priority > v.priority || (w.priority == v.priority && wi < vi)))
                .then_some(w)
        }) {
            out.add(
                Sev::Error,
                "voice",
                &v.id,
                format!(
                    "never a speaker's voice: `{}` has no conditions and priority {} ≥ {}",
                    w.id, w.priority, v.priority
                ),
            );
        }
    }
    for l in &b.talk_lines {
        check_conditions(out, &l.id, "when", &l.when);
        cond_refs(b, out, &l.id, None, &l.when);
        line_voice_checks(b, out, l);
    }
}

pub(super) fn check_cast(b: &ContentBundle, out: &mut Out, item: &str, c: &Cast) {
    match c {
        Cast::Occupation { occupations } => {
            for o in occupations {
                if b.occupation(o).is_none() {
                    out.add(
                        Sev::Error,
                        "unknown-ref",
                        item,
                        format!("cast names unknown occupation `{o}`"),
                    );
                }
            }
        }
        Cast::Law { templates, .. } => {
            for t in templates {
                if !b.policies.contains_key(t) {
                    out.add(
                        Sev::Error,
                        "unknown-ref",
                        item,
                        format!("cast names unknown policy template `{t}`"),
                    );
                }
            }
        }
        _ => {}
    }
}

/// The ids a list of conditions names exist (`any` alternatives too), and,
/// where the item has a cast (`roles`), the roles its conditions name are
/// cast.
pub(super) fn cond_refs(
    b: &ContentBundle,
    out: &mut Out,
    item: &str,
    roles: Option<&[&str]>,
    cs: &[Condition],
) {
    use Condition as C;
    let talk_roles = b.talk_roles();
    let unknown = |out: &mut Out, what: &str, id: &str| {
        out.add(
            Sev::Error,
            "unknown-ref",
            item,
            format!("{what} `{id}` does not exist"),
        );
    };
    for c in cs {
        if let Some(roles) = roles {
            for r in c.slots() {
                if !roles.contains(&r) {
                    out.add(
                        Sev::Error,
                        "unknown-ref",
                        item,
                        format!("a condition names role `{r}`, which the item does not cast"),
                    );
                }
            }
        }
        match c {
            C::Crisis {
                kind: Some(kind), ..
            } if !b.crises.iter().any(|x| &x.id == kind) => unknown(out, "crisis", kind),
            C::Concern { top, .. } => {
                for k in c.worry().map(|w| w.concerns).unwrap_or_default() {
                    if !b.concerns.iter().any(|x| &x.id == k) {
                        unknown(out, "concern", k);
                    }
                }
                if *top == Some(0) {
                    out.add(
                        Sev::Error,
                        "unsatisfiable",
                        item,
                        "a concern among the top 0 never holds",
                    );
                }
            }
            C::Trait { traits, .. } => {
                for t in traits {
                    if b.trait_def(t).is_none() {
                        unknown(out, "trait", t);
                    }
                }
            }
            C::LawInForce { templates, .. } => {
                for t in templates {
                    if !b.policies.contains_key(t) {
                        unknown(out, "policy template", t);
                    }
                }
            }
            C::Goal { goal, .. } if b.goal(goal).is_none() => unknown(out, "goal", goal),
            C::Remembers { kinds, .. } => {
                for k in kinds {
                    if !mandate_content::MEMORY_KINDS.contains(&k.as_str()) {
                        unknown(out, "memory kind", k);
                    }
                }
            }
            C::Occupation { occupations, .. } => {
                for o in occupations {
                    if !talk_roles.contains(o) {
                        unknown(out, "role", o);
                    }
                }
            }
            C::Present { occupations } => {
                for o in occupations {
                    if b.occupation(o).is_none() {
                        unknown(out, "occupation", o);
                    }
                }
            }
            C::Voice { voices } => {
                for v in voices {
                    if b.talk_voice(v).is_none() {
                        unknown(out, "voice", v);
                    }
                }
            }
            C::Any { of } => cond_refs(b, out, item, roles, of),
            _ => {}
        }
    }
}

/// Voice and role problems of a line: a closer's voice (or an opener's
/// `voice` condition) whose speakers can never have the line's roles, a
/// substance line for roles its topic is never offered to, and the line's
/// conditions together with the voice's or the topic's.
pub(super) fn line_voice_checks(b: &ContentBundle, out: &mut Out, l: &TalkLineDef) {
    let mine = speaker_roles(&l.when);
    let voices: Vec<&str> = match (&l.voice, l.part) {
        (Some(v), TalkPart::Closer) => vec![v.as_str()],
        _ => l
            .when
            .iter()
            .filter_map(|c| match c {
                Condition::Voice { voices } => {
                    Some(voices.iter().map(|x| x.as_str()).collect::<Vec<_>>())
                }
                _ => None,
            })
            .next()
            .unwrap_or_default(),
    };
    if !voices.is_empty() {
        let possible: Vec<&str> = voices
            .iter()
            .copied()
            .filter(|v| {
                let Some(vd) = b.talk_voice(v) else {
                    return false;
                };
                let vr = speaker_roles(&vd.when);
                match (&mine, vr) {
                    (Some(a), Some(bb)) => a.intersection(&bb).next().is_some(),
                    _ => true,
                }
            })
            .collect();
        if possible.is_empty() {
            out.add(
                Sev::Error,
                "voice",
                &l.id,
                format!(
                    "speakers with voice {} never have this line's roles ({})",
                    voices.join(" / "),
                    mine.as_ref()
                        .map(|m| m.iter().cloned().collect::<Vec<_>>().join(", "))
                        .unwrap_or_default()
                ),
            );
        }
        for v in &voices {
            if let Some(vd) = b.talk_voice(v) {
                let mut all = vd.when.clone();
                all.extend(
                    l.when
                        .iter()
                        .filter(|c| !matches!(c, Condition::Voice { .. }))
                        .cloned(),
                );
                check_conditions(out, &l.id, &format!("with voice {v}"), &all);
            }
        }
    }
    if let Some(t) = l.topic.as_deref().and_then(|k| b.talk_topic(k)) {
        if l.part == TalkPart::Substance
            && !t.offered.roles.is_empty()
            && let Some(m) = &mine
            && !m.iter().any(|r| t.offered.roles.contains(r))
        {
            out.add(
                Sev::Error,
                "voice",
                &l.id,
                format!(
                    "its roles ({}) are never offered topic `{}` ({})",
                    m.iter().cloned().collect::<Vec<_>>().join(", "),
                    t.id,
                    t.offered.roles.join(", ")
                ),
            );
        }
        let mut all = t.offered.when.clone();
        all.extend(l.when.iter().cloned());
        check_conditions(out, &l.id, &format!("with topic {}", t.id), &all);
    }
    if let Some(lead) = &l.lead
        && let LeadToDef::PlaceKind { kind } = &lead.to
        && b.place_kind(kind).is_none()
    {
        out.add(
            Sev::Error,
            "unknown-ref",
            &l.id,
            format!("lead to unknown place kind `{kind}`"),
        );
    }
}

// --- Simulation: what fired in N careers -------------------------------------------------
