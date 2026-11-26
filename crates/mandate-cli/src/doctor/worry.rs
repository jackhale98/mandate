//! Worry and subject rules: lines asserting a worry their conditions do not make real.

use super::*;

/// Phrases that assert one particular worry, by concern. A trailing `*`
/// matches a word's start ("redundan*": redundancy, redundancies); the rest
/// match whole words.
pub(super) const WORRY_WORDS: &[(&str, &[&str])] = &[
    (
        "concern.housing",
        &[
            "rent",
            "rents",
            "landlord*",
            "one-bed",
            "housing",
            "mortgage*",
        ],
    ),
    (
        "concern.jobs",
        &[
            "laid off",
            "lay-off*",
            "layoff*",
            "redundanc*",
            "out of work",
            "jobs gone",
            "unemploy*",
        ],
    ),
    (
        "concern.environment",
        &["pollution", "the air", "smoke", "fumes", "wheez*", "foam"],
    ),
    (
        "concern.taxes",
        &["the rates", "rates bill", "property tax", "tax bills"],
    ),
    (
        "concern.services",
        &["bins", "library", "waiting room", "late bus", "glue sticks"],
    ),
    (
        "concern.schools_health",
        &[
            "for an appointment",
            "see a doctor",
            "lost two teachers",
            "thirty-four children",
        ],
    ),
    (
        "concern.cost_of_living",
        &["same basket", "prices went up", "sitting in coats"],
    ),
    (
        "concern.newcomers",
        &["faces on this street", "new families", "new people moving"],
    ),
    (
        "concern.young_leaving",
        &["nothing for them here", "young ones left", "pensioners now"],
    ),
    (
        "concern.upkeep",
        &["tyres", "street lights", "drains back up"],
    ),
    (
        "concern.trust",
        &["in front of a judge", "looking after themselves"],
    ),
    (
        "concern.development",
        &[
            "cranes everywhere",
            "going to be flats",
            "the works have been",
        ],
    ),
    (
        "concern.recovery",
        &[
            "tarpaulin",
            "tarps",
            "builder*",
            "insurance",
            "rebuild*",
            "clearing up",
            "the disaster",
            "when it hit",
        ],
    ),
];

/// The worries a text asserts (by `WORRY_WORDS`), with the phrase that says so.
pub(super) fn asserted_worries(text: &str) -> BTreeMap<&'static str, &'static str> {
    let t = text.to_lowercase();
    let mut out = BTreeMap::new();
    for (concern, words) in WORRY_WORDS {
        for w in *words {
            let (w_str, prefix) = match w.strip_suffix('*') {
                Some(s) => (s, true),
                None => (*w, false),
            };
            let found = t.match_indices(w_str).any(|(i, _)| {
                let before = t[..i].chars().next_back();
                let after = t[i + w_str.len()..].chars().next();
                !before.is_some_and(char::is_alphabetic)
                    && (prefix || !after.is_some_and(char::is_alphabetic))
            });
            if found {
                out.entry(*concern).or_insert(*w);
                break;
            }
        }
    }
    out
}

/// The worries an indicator, a crisis or a town event speaks for.
pub(super) fn indicator_worry(i: mandate_content::Indicator) -> &'static str {
    use mandate_content::Indicator as I;
    match i {
        I::RentBurden => "concern.housing",
        I::Unemployment | I::UnemploymentRise | I::JobLoss => "concern.jobs",
        I::Pollution => "concern.environment",
        I::TaxBurden => "concern.taxes",
        I::BudgetStress | I::ServiceShortfall => "concern.services",
        I::RecentDisaster => "concern.recovery",
        I::CostOfLiving => "concern.cost_of_living",
        I::Arrivals => "concern.newcomers",
        I::YoungLeaving | I::OldAgeShare => "concern.young_leaving",
        I::CareShortfall => "concern.schools_health",
        I::UpkeepShortfall | I::StreetWear => "concern.upkeep",
        I::Scandals => "concern.trust",
        I::Construction | I::WorksDisruption => "concern.development",
    }
}

pub(super) fn crisis_worry(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "crisis.plant_closure" => "concern.jobs",
        "crisis.housing_squeeze" => "concern.housing",
        "crisis.fiscal_hole" => "concern.services",
        "crisis.disaster_recovery" => "concern.recovery",
        _ => return None,
    })
}

/// The worries a conversation line's conditions make true (a `tells` of the
/// town's or a neighbourhood's worries is the speaker's top worry, whatever
/// it is: it backs `$concern`, never a particular worry).
pub(super) fn cond_worries(cs: &[Condition], out: &mut BTreeSet<String>) {
    use Condition as C;
    for c in cs {
        match c {
            C::Concern { .. } => {
                out.extend(c.worry().into_iter().flat_map(|w| w.concerns).cloned());
            }
            C::Indicator { indicator, .. } => {
                out.insert(indicator_worry(*indicator).into());
            }
            C::Crisis { kind: Some(k), .. } => out.extend(crisis_worry(k).map(String::from)),
            C::PlantClosure { .. } => {
                out.insert("concern.jobs".into());
            }
            C::Disaster { .. } => {
                out.insert("concern.recovery".into());
            }
            C::Any { of } => cond_worries(of, out),
            _ => {}
        }
    }
}

/// A line that asserts a particular worry ("It's the rent", "another lot
/// laid off") must be said only where that worry is real: a `concern`
/// condition naming it, an indicator, a crisis or a town event behind it
/// (for overheard lines, the line's `concern`; for a dilemma's situation,
/// its triggers). A line that asserts worries none of which its conditions
/// back is a `worry-unconditioned` warning. Lines that name the worry
/// through `$concern` are true by construction and never asserted here.
pub(super) fn worry_checks(b: &ContentBundle, out: &mut Out) {
    let msgs = messages(&b.locale_text("en"));
    let check = |out: &mut Out, item: &str, key: &str, backed: &BTreeSet<String>| {
        let Some(text) = msgs.get(key) else { return };
        let said = asserted_worries(text);
        if said.is_empty() || said.keys().any(|c| backed.contains(*c)) {
            return;
        }
        let list: Vec<String> = said
            .iter()
            .map(|(c, w)| format!("{} (\"{}\")", c, w.trim_end_matches('*')))
            .collect();
        out.add(
            Sev::Warn,
            "worry-unconditioned",
            item,
            format!(
                "`{key}` asserts {} but nothing in its conditions makes that worry real",
                list.join(", ")
            ),
        );
    };
    for l in &b.talk_lines {
        let mut backed = BTreeSet::new();
        cond_worries(&l.when, &mut backed);
        check(out, &l.id, &l.key, &backed);
    }
    for o in &b.overheard {
        let mut backed: BTreeSet<String> = o.concern.iter().cloned().collect();
        cond_worries(&o.when, &mut backed);
        check(out, &o.id, &o.key, &backed);
    }
    for d in &b.dilemmas {
        let mut backed = BTreeSet::new();
        cond_worries(&d.triggers, &mut backed);
        check(out, &d.id, &d.situation_key, &backed);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::doctor::testkit::*;

    #[test]
    fn a_worry_asserted_without_a_worry_condition_is_found() {
        let mut s = base_sources();
        s.push(Source {
            path: "base/planted_lines.jsonc".into(),
            text: r#"{ "kind": "talk_lines", "items": [
  { "id": "line.planted.rent", "part": "substance", "topic": "topic.worries",
    "key": "talk-planted-rent" },
  { "id": "line.planted.rent_ok", "part": "substance", "topic": "topic.worries",
    "key": "talk-planted-rent-ok",
    "when": [ { "type": "concern", "of": "town", "concerns": ["concern.housing"] } ] },
  { "id": "line.planted.any", "part": "substance", "topic": "topic.worries",
    "key": "talk-planted-any",
    "when": [ { "type": "tells", "subject": { "type": "town_concerns" }, "min": "heard" } ] }
] }"#
                .into(),
        });
        s.push(Source {
            path: "base/locale/en/planted.ftl".into(),
            text: "talk-planted-rent = \"It's the rent. Everyone's landlord put it up.\"\n\
                   talk-planted-rent-ok = \"It's the rent. Everyone's landlord put it up.\"\n\
                   talk-planted-any = \"From what I hear, it's { $concern_talk }.\"\n"
                .into(),
        });
        let r = diagnose(&s, &quick());
        assert!(
            r.has("worry-unconditioned", "line.planted.rent"),
            "{}",
            human(&r)
        );
        assert!(!r.has("worry-unconditioned", "line.planted.rent_ok"));
        assert!(!r.has("worry-unconditioned", "line.planted.any"));
        // The base pack's own lines all have their worries behind them.
        let base = diagnose(&base_sources(), &quick());
        let unbacked: Vec<_> = base
            .findings
            .iter()
            .filter(|f| f.check == "worry-unconditioned")
            .collect();
        assert!(unbacked.is_empty(), "{unbacked:#?}");
    }
}
