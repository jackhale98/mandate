//! Role coverage of conversations (M6 note 26): answers come from the
//! speaker's role where it bears on the question, so each topic that
//! informs wants, for each role group (`talk_params.role_groups`), at least
//! one informative line written for that group: a substance line of the
//! topic that is not a safe answer or a miss, whose `occupation` condition
//! names only roles of the group. A topic and group without one is
//! reported (`role-coverage`, a warning: the group's speakers answer with
//! the general lines or the safe answer). A role in no group is reported
//! too (`role-ungrouped`).

use super::*;
use mandate_content::conversation::TalkMechanic;

/// The roles a line is written for: its positive `occupation` condition on
/// the speaker (`None`: written for nobody in particular).
fn written_for(l: &TalkLineDef) -> Option<&[String]> {
    l.when.iter().find_map(|c| match c {
        Condition::Occupation {
            slot,
            occupations,
            is: true,
        } if slot == "speaker" && !occupations.is_empty() => Some(occupations.as_slice()),
        _ => None,
    })
}

pub(super) fn role_coverage_checks(b: &ContentBundle, out: &mut Out) {
    let groups = &b.talk.role_groups;
    if groups.is_empty() {
        return;
    }
    for role in b.talk_roles() {
        if !groups.iter().any(|g| g.roles.contains(&role)) {
            out.add(
                Sev::Warn,
                "role-ungrouped",
                "talk_params",
                format!("role `{role}` is in no role group, so no lines are written for it"),
            );
        }
    }
    for t in b
        .talk_topics
        .iter()
        .filter(|t| t.mechanic == TalkMechanic::Inform)
    {
        for g in groups {
            let covered = b.talk_lines.iter().any(|l| {
                l.part == TalkPart::Substance
                    && l.topic.as_deref() == Some(t.id.as_str())
                    && !l.fallback
                    && !l.miss
                    && written_for(l).is_some_and(|rs| rs.iter().all(|r| g.roles.contains(r)))
            });
            if !covered {
                out.add(
                    Sev::Warn,
                    "role-coverage",
                    &t.id,
                    format!(
                        "no informative line written for `{}` ({}): they answer with the general lines or the safe answer",
                        g.id,
                        g.roles.join(", ")
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::doctor::testkit::*;

    /// The (topic, group) pairs reported for a pack.
    fn gaps(r: &Report) -> Vec<(String, String)> {
        r.findings
            .iter()
            .filter(|f| f.check == "role-coverage")
            .map(|f| {
                let group = f.message.split('`').nth(1).unwrap_or_default().to_string();
                (f.item.clone(), group)
            })
            .collect()
    }

    #[test]
    fn a_group_with_no_informative_line_of_its_own_is_reported() {
        // A planted group nobody has written for: students, taken out of
        // the residents.
        let mut s = base_sources();
        let f = s
            .iter_mut()
            .find(|x| x.path == "base/conversations/params.jsonc")
            .unwrap();
        assert!(f.text.contains(r#""occupation.student", "#));
        f.text = f.text.replacen(r#""occupation.student", "#, "", 1);
        assert!(f.text.contains(r#""role_groups": ["#));
        f.text = f.text.replacen(
            r#""role_groups": ["#,
            r#""role_groups": [ { "id": "group.planted", "roles": ["occupation.student"] },"#,
            1,
        );
        let r = diagnose(&s, &quick());
        let found = gaps(&r);
        for topic in ["topic.worries", "topic.who_runs"] {
            assert!(
                found.contains(&(topic.to_string(), "group.planted".to_string())),
                "{topic}: {found:?}\n{}",
                human(&r)
            );
        }
        assert!(
            r.findings
                .iter()
                .filter(|f| f.check == "role-coverage")
                .all(|f| f.severity == Sev::Warn)
        );
    }

    #[test]
    fn the_base_pack_covers_every_group_but_those_who_cannot_know_who_runs_things() {
        let r = diagnose(&base_sources(), &quick());
        let mut found = gaps(&r);
        found.sort();
        // Nobody in these groups knows who runs things (`speaker_knows`):
        // their role-written answers are safe answers that name the visible
        // power and point to who would know.
        let expected: Vec<(String, String)> = ["group.care", "group.residents", "group.trade"]
            .iter()
            .map(|g| ("topic.who_runs".to_string(), g.to_string()))
            .collect();
        assert_eq!(found, expected, "{}", human(&r));
        assert!(
            !r.findings.iter().any(|f| f.check == "role-ungrouped"),
            "{}",
            human(&r)
        );
    }
}
