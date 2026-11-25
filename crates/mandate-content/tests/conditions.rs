#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! One condition language (DEC-098 §5, task U1): the former shapes of the
//! three languages (narrative, situation, heard) still parse into the one
//! `Condition`, files written in them still load, and the one validator's
//! table refuses a variant where it is not allowed (`knows` where something
//! is decided, a slot where none is bound, a conversation, cast, whole-state
//! or place reading where there is none, a local-only variant in a
//! non-local item). `cargo test -p mandate-content --features fs --test
//! conditions -- --nocapture` prints the table.
use mandate_content::condition::{CondTier, ConditionUse, Site, check};
use mandate_content::conversation::{ConcernOf, SubjectRef};
use mandate_content::knowledge::Level;
use mandate_content::{
    Condition, ContentBundle, HeardSubject, Severity, Source, SubjectDef, TierKey, WaterFeature,
    bundle::read_dir_sources, load,
};
use std::path::Path;

fn sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn base() -> ContentBundle {
    load(&sources()).unwrap().0
}

fn parse(s: &str) -> Condition {
    serde_json::from_str(&mandate_content::jsonc::to_strict_json(s))
        .unwrap_or_else(|e| panic!("{s}: {e}"))
}

/// Errors of a load with extra files.
fn errors_with(extra: &[(&str, &str)]) -> Vec<String> {
    let mut src = sources();
    for (path, text) in extra {
        src.push(Source {
            path: (*path).into(),
            text: (*text).into(),
        });
    }
    match load(&src) {
        Ok(_) => Vec::new(),
        Err(issues) => issues
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.to_string())
            .collect(),
    }
}

// --- The former shapes ---------------------------------------------------------------

/// The narrative language's spellings: `role` for the slot, `live_crisis`,
/// a one-concern `concern` with `top`, `industry` with drivers.
#[test]
fn narrative_spellings_parse() {
    assert_eq!(
        parse(r#"{ "type": "live_crisis", "min_severity": 0.3 }"#),
        Condition::Crisis {
            kind: None,
            min_severity: 0.3
        }
    );
    assert_eq!(
        parse(r#"{ "type": "crisis", "kind": "crisis.plant_closure", "min_severity": 0.2 }"#),
        Condition::Crisis {
            kind: Some("crisis.plant_closure".into()),
            min_severity: 0.2
        }
    );
    let c = parse(r#"{ "type": "concern", "concern": "concern.jobs", "top": 2 }"#);
    let w = c.worry().unwrap();
    assert_eq!(w.top, 2);
    assert_eq!(w.of, ConcernOf::Town);
    assert_eq!(w.concerns, vec![&"concern.jobs".to_string()]);
    assert_eq!(
        parse(r#"{ "type": "industry", "drivers": ["industrial"] }"#),
        Condition::Industry {
            drivers: Some(vec![mandate_content::SectorDriver::Industrial])
        }
    );
    for (old, slot) in [
        (
            r#"{ "type": "opinion", "role": "holder", "min": 10 }"#,
            "holder",
        ),
        (r#"{ "type": "favour", "role": "creditor" }"#, "creditor"),
        (
            r#"{ "type": "remembers", "role": "holder", "kinds": ["met"] }"#,
            "holder",
        ),
        (
            r#"{ "type": "personality", "role": "r", "axis": "warmth", "min": 0.5 }"#,
            "r",
        ),
        (
            r#"{ "type": "trait", "role": "r", "traits": ["trait.vain"] }"#,
            "r",
        ),
        (
            r#"{ "type": "goal", "role": "steward", "goal": "goal.x" }"#,
            "steward",
        ),
        (
            r#"{ "type": "party_line_against_ward", "role": "bill" }"#,
            "bill",
        ),
        (r#"{ "type": "can_vote", "role": "bill" }"#, "bill"),
        (r#"{ "type": "sponsored_law", "role": "law" }"#, "law"),
    ] {
        assert_eq!(parse(old).slots(), vec![slot], "{old}");
    }
    for old in [
        r#"{ "type": "election_within", "days": 60 }"#,
        r#"{ "type": "campaigning" }"#,
        r#"{ "type": "office", "role": "councillor" }"#,
        r#"{ "type": "office", "posts": ["any"] }"#,
        r#"{ "type": "track", "track": "legal_jeopardy", "max": 40 }"#,
        r#"{ "type": "money", "min": 100 }"#,
        r#"{ "type": "months_in_play", "min": 2 }"#,
        r#"{ "type": "law_in_force", "templates": ["policy.transit"], "absent": true }"#,
        r#"{ "type": "knows", "subject": { "type": "broker" } }"#,
        r#"{ "type": "answered", "template": "dilemma.x", "option": "a", "min_days": 30 }"#,
    ] {
        parse(old);
    }
    assert_eq!(
        parse(r#"{ "type": "knows", "subject": { "type": "broker" } }"#),
        Condition::Knows {
            subject: SubjectDef::Broker,
            min: Level::Heard
        }
    );
}

/// The situation language's spellings: `slot`, the conversation's `concern`
/// (`of`, `concerns`), the conversation-only variants.
#[test]
fn situation_spellings_parse() {
    let c = parse(
        r#"{ "type": "concern", "of": "speaker", "concerns": ["concern.jobs", "concern.housing"] }"#,
    );
    let w = c.worry().unwrap();
    assert_eq!((w.top, w.of), (1, ConcernOf::Speaker));
    assert_eq!(w.concerns.len(), 2);
    for s in [
        r#"{ "type": "indicator", "indicator": "unemployment", "min": 0.08 }"#,
        r#"{ "type": "plant_closure", "within_months": 12 }"#,
        r#"{ "type": "disaster", "within_months": 6 }"#,
        r#"{ "type": "occupation", "slot": "speaker", "occupations": ["occupation.nurse"] }"#,
        r#"{ "type": "trust", "slot": "speaker", "min": 0.5 }"#,
        r#"{ "type": "is_broker", "slot": "speaker" }"#,
        r#"{ "type": "is_rival", "slot": "about", "is": false }"#,
        r#"{ "type": "tells", "subject": { "type": "town_concerns" }, "min": "heard" }"#,
        r#"{ "type": "player_knows", "subject": { "type": "broker" }, "max": "heard" }"#,
        r#"{ "type": "voice", "voices": ["voice.blunt"] }"#,
        r#"{ "type": "about", "kinds": ["place"], "workplace": true }"#,
        r#"{ "type": "any", "of": [ { "type": "disaster", "within_months": 6 } ] }"#,
    ] {
        parse(s);
    }
    assert_eq!(
        parse(r#"{ "type": "tells", "subject": { "type": "broker" }, "min": "known" }"#),
        Condition::Tells {
            subject: SubjectRef::Broker,
            min: Level::Known
        }
    );
}

/// The heard language's spellings: unit variants, `crisis` without a kind,
/// `industry` without drivers.
#[test]
fn heard_spellings_parse() {
    assert_eq!(parse(r#"{ "type": "dry" }"#), Condition::Dry);
    assert_eq!(parse(r#"{ "type": "broker" }"#), Condition::Broker);
    assert_eq!(parse(r#"{ "type": "rival" }"#), Condition::Rival);
    assert_eq!(parse(r#"{ "type": "headline" }"#), Condition::Headline);
    assert_eq!(
        parse(r#"{ "type": "industry" }"#),
        Condition::Industry { drivers: None }
    );
    assert_eq!(
        parse(r#"{ "type": "crisis" }"#),
        Condition::Crisis {
            kind: None,
            min_severity: 0.0
        }
    );
    assert_eq!(
        parse(r#"{ "type": "water", "feature": "river" }"#),
        Condition::Water {
            feature: WaterFeature::River
        }
    );
    assert_eq!(
        parse(r#"{ "type": "scandal", "about": "rival" }"#),
        Condition::Scandal {
            about: HeardSubject::Rival
        }
    );
    for s in [
        r#"{ "type": "player_awareness", "max": 0.12 }"#,
        r#"{ "type": "player_approval", "min": -0.12, "max": 0.12 }"#,
        r#"{ "type": "rival_record", "min": 0.55 }"#,
        r#"{ "type": "election_within", "days": 45 }"#,
    ] {
        parse(s);
    }
}

/// Files written in each former shape still load and validate: a dilemma in
/// the narrative spellings, overheard lines in the heard spellings, and a
/// situational policy in the situation spellings (the base pack itself is
/// in all three).
#[test]
fn old_files_still_load() {
    let base_errors = errors_with(&[]);
    assert!(base_errors.is_empty(), "{base_errors:?}");
    let dilemma = r#"{"kind":"dilemmas","items":[{"id":"dilemma.u1_old","category":"significant",
        "title_key":"dilemma-favour-called-in","situation_key":"dilemma-favour-called-in-situation",
        "roles":[ { "id": "creditor", "cast": { "type": "creditor" } },
                  { "id": "bill", "cast": { "type": "bill", "sponsor": "other", "sponsored_by": "creditor" } } ],
        "triggers": [
          { "type": "live_crisis", "min_severity": 0.1 },
          { "type": "concern", "concern": "concern.jobs", "top": 3 },
          { "type": "industry", "drivers": ["industrial"] },
          { "type": "favour", "role": "creditor" },
          { "type": "opinion", "role": "creditor", "min": -50 },
          { "type": "can_vote", "role": "bill" },
          { "type": "party_line_against_ward", "role": "bill" },
          { "type": "office", "role": "councillor" },
          { "type": "months_in_play", "min": 1 }
        ],
        "cooldown_days":30,
        "options":[
          {"id":"a","label_key":"dilemma-favour-called-in-pay","aftermath_key":"dilemma-favour-called-in-pay-after",
           "effects":[ { "type": "vote", "role": "bill", "vote": "yes" } ]},
          {"id":"b","default":true,"label_key":"dilemma-favour-called-in-refuse","aftermath_key":"dilemma-favour-called-in-refuse-after",
           "effects":[ { "type": "vote", "role": "bill", "vote": "no" } ]}]}]}"#;
    let e = errors_with(&[("aa_u1/dilemmas.jsonc", dilemma)]);
    assert!(e.is_empty(), "{e:?}");
    let overheard = r#"{"kind":"overheard","items":[
        {"id":"overheard.u1_old","key":"heard-jobs-notices","when":[
          { "type": "water", "feature": "any" }, { "type": "crisis" }, { "type": "industry" },
          { "type": "broker" }, { "type": "rival" }, { "type": "headline" },
          { "type": "player_awareness", "max": 0.9 }, { "type": "scandal", "about": "player" },
          { "type": "any", "of": [ { "type": "dry" }, { "type": "election_within", "days": 90 } ] }
        ]}]}"#;
    let e = errors_with(&[("aa_u1/overheard.jsonc", overheard)]);
    assert!(e.is_empty(), "{e:?}");
}

/// The generated schema accepts the old spellings the parser accepts.
#[test]
fn the_schema_accepts_the_old_spellings() {
    let s = serde_json::to_string(&mandate_content::content_schema()).unwrap();
    assert!(s.contains(r#""const":"live_crisis""#), "live_crisis");
    let v = mandate_content::content_schema();
    let defs = v.get("$defs").unwrap();
    let cond = defs.get("Condition").unwrap().get("oneOf").unwrap();
    let opinion = cond
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x.pointer("/properties/type/const").and_then(|c| c.as_str()) == Some("opinion"))
        .unwrap();
    assert!(opinion.pointer("/properties/role").is_some());
    assert!(opinion.pointer("/properties/slot").is_some());
}

// --- The one validator's table ---------------------------------------------------------

fn refusals(b: &ContentBundle, site: &Site<'_>, c: &Condition) -> Vec<String> {
    let mut issues = Vec::new();
    check(b, site, c, &mut issues);
    issues
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.message)
        .collect()
}

#[test]
fn the_validator_refuses_a_variant_where_it_is_not_allowed() {
    let b = base();
    const CAST: &[&str] = &["who", "bill"];
    const TALK: &[&str] = &["speaker", "about", "player"];
    let (cast, talk) = (CAST, TALK);
    let at = |u: ConditionUse, slots: &'static [&'static str]| Site::new("item", u, slots);
    let knows = parse(r#"{ "type": "knows", "subject": { "type": "broker" } }"#);
    // `knows` frames what the player is shown, never whether something
    // happens (K6, DEC-091): refused in a trigger, allowed in a scene variant.
    let e = refusals(&b, &at(ConditionUse::DilemmaTrigger, cast), &knows);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("DEC-091"), "{e:?}");
    assert!(refusals(&b, &at(ConditionUse::SceneVariant, cast), &knows).is_empty());
    for u in [
        ConditionUse::GoalCondition,
        ConditionUse::ArcRaise,
        ConditionUse::ArcEdge,
        ConditionUse::Ask,
    ] {
        assert_eq!(refusals(&b, &at(u, cast), &knows).len(), 1, "{u:?}");
    }
    // The ledger and what a speaker would tell, also nested in an `any`.
    let tells = parse(r#"{ "type": "tells", "subject": { "type": "broker" }, "min": "heard" }"#);
    let nested = Condition::Any {
        of: vec![
            parse(r#"{ "type": "disaster", "within_months": 6 }"#),
            parse(r#"{ "type": "player_knows", "subject": { "type": "broker" }, "max": "heard" }"#),
        ],
    };
    for c in [&tells, &nested] {
        let e = refusals(&b, &at(ConditionUse::PolicyUnlock, &[]), c);
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(
            e[0].contains("never decides whether something happens"),
            "{e:?}"
        );
        assert!(refusals(&b, &at(ConditionUse::TalkLine, talk), c).is_empty());
    }
    // A slot-relative variant where no slot is bound.
    let opinion = parse(r#"{ "type": "opinion", "slot": "speaker", "min": 0 }"#);
    for u in [
        ConditionUse::PolicyUnlock,
        ConditionUse::Overheard,
        ConditionUse::VisitVerb,
        ConditionUse::FamilyTrigger,
    ] {
        let e = refusals(&b, &at(u, &[]), &opinion);
        assert!(
            e.len() == 1 && e[0].contains("this condition reads a slot"),
            "{u:?}: {e:?}"
        );
    }
    // A slot that is not bound here.
    let e = refusals(&b, &at(ConditionUse::DilemmaTrigger, cast), &opinion);
    assert!(
        e.iter().any(|x| x.contains("unknown role `speaker`")),
        "{e:?}"
    );
    let e = refusals(
        &b,
        &at(ConditionUse::TalkLine, &["speaker", "player"]),
        &parse(r#"{ "type": "trait", "slot": "about", "traits": ["trait.vain"] }"#),
    );
    assert!(
        e.iter().any(|x| x.contains("unknown slot `about`")),
        "{e:?}"
    );
    // A conversation's reading outside a conversation.
    let voice = parse(r#"{ "type": "voice", "voices": ["voice.blunt"] }"#);
    let e = refusals(&b, &at(ConditionUse::DilemmaTrigger, cast), &voice);
    assert!(
        e.len() == 1 && e[0].contains("reads a conversation"),
        "{e:?}"
    );
    assert!(refusals(&b, &at(ConditionUse::TalkLine, talk), &voice).is_empty());
    // A cast's bill outside a cast.
    let can_vote = parse(r#"{ "type": "can_vote", "slot": "bill" }"#);
    let e = refusals(&b, &at(ConditionUse::TalkLine, talk), &can_vote);
    assert!(e.len() == 1 && e[0].contains("has no cast"), "{e:?}");
    assert!(refusals(&b, &at(ConditionUse::DilemmaTrigger, cast), &can_vote).is_empty());
    // The event log and people's routines inside the simulation.
    let closure = parse(r#"{ "type": "plant_closure", "within_months": 12 }"#);
    let e = refusals(&b, &at(ConditionUse::DilemmaTrigger, cast), &closure);
    assert!(e.len() == 1 && e[0].contains("whole game state"), "{e:?}");
    assert!(refusals(&b, &at(ConditionUse::PolicyUnlock, &[]), &closure).is_empty());
    // A place's reading where there is no place.
    let dry = parse(r#"{ "type": "dry" }"#);
    let e = refusals(&b, &at(ConditionUse::PolicyUnlock, &[]), &dry);
    assert!(
        e.len() == 1 && e[0].contains("where it is said or done"),
        "{e:?}"
    );
    assert!(refusals(&b, &at(ConditionUse::Overheard, &[]), &dry).is_empty());
    // Ranges and ids, wherever it is used.
    let e = refusals(
        &b,
        &at(ConditionUse::Overheard, &[]),
        &parse(r#"{ "type": "player_approval", "min": 0.5, "max": 0.2 }"#),
    );
    assert!(e.iter().any(|x| x.contains("min is above max")), "{e:?}");
    let e = refusals(
        &b,
        &at(ConditionUse::VisitVerb, &[]),
        &parse(r#"{ "type": "present", "occupations": ["occupation.pope"] }"#),
    );
    assert!(e.iter().any(|x| x.contains("unknown occupation")), "{e:?}");
}

/// A non-local item (a regional template, DEC-098) uses tier-generic
/// conditions only.
#[test]
fn a_local_only_variant_is_refused_in_a_non_local_item() {
    let b = base();
    let regional =
        Site::new("policy.regional", ConditionUse::PolicyUnlock, &[]).tier(TierKey::Regional);
    let crisis = parse(r#"{ "type": "crisis", "kind": "crisis.plant_closure" }"#);
    assert_eq!(crisis.tier(), CondTier::Local);
    let e = refusals(&b, &regional, &crisis);
    assert!(e.len() == 1 && e[0].contains("DEC-098"), "{e:?}");
    let local = Site::new("policy.local", ConditionUse::PolicyUnlock, &[]);
    assert!(refusals(&b, &local, &crisis).is_empty());
    for generic in [
        r#"{ "type": "months_in_play", "min": 2 }"#,
        r#"{ "type": "phase", "phase": "campaign" }"#,
        r#"{ "type": "holds_power", "power": "veto" }"#,
    ] {
        let c = parse(generic);
        assert_eq!(c.tier(), CondTier::Generic, "{generic}");
        assert!(refusals(&b, &regional, &c).is_empty(), "{generic}");
    }
    // In an `any`, each alternative is checked.
    let any = Condition::Any {
        of: vec![parse(r#"{ "type": "months_in_play", "min": 2 }"#), crisis],
    };
    assert_eq!(any.tier(), CondTier::Local);
    assert_eq!(refusals(&b, &regional, &any).len(), 1);
    // Through the content pipeline: a regional situational template.
    let mut src = sources();
    let f = src
        .iter_mut()
        .find(|s| s.path.ends_with("policies/situational.jsonc"))
        .unwrap();
    f.text = f
        .text
        .replacen(r#""tier": "local""#, r#""tier": "regional""#, 1);
    let errs = match load(&src) {
        Ok(_) => Vec::new(),
        Err(i) => i.into_iter().map(|x| x.to_string()).collect(),
    };
    assert!(errs.iter().any(|x| x.contains("DEC-098")), "{errs:?}");
}

/// The table: every use against every variant's needs (printed with
/// `--nocapture`, for EXTENDING.md and the report), and every use has a
/// label.
#[test]
fn the_table() {
    let samples = [
        r#"{ "type": "crisis" }"#,
        r#"{ "type": "plant_closure", "within_months": 6 }"#,
        r#"{ "type": "dry" }"#,
        r#"{ "type": "present", "occupations": ["occupation.nurse"] }"#,
        r#"{ "type": "day_part", "parts": ["morning"] }"#,
        r#"{ "type": "opinion", "slot": "x", "min": 0 }"#,
        r#"{ "type": "can_vote", "slot": "x" }"#,
        r#"{ "type": "knows", "subject": { "type": "broker" } }"#,
        r#"{ "type": "tells", "subject": { "type": "broker" }, "min": "heard" }"#,
        r#"{ "type": "voice", "voices": ["voice.blunt"] }"#,
    ];
    let mut out = String::from("| variant | tier |");
    for u in ConditionUse::ALL {
        out.push_str(&format!(" {u:?} |"));
    }
    out.push('\n');
    for s in samples {
        let c = parse(s);
        out.push_str(&format!("| {} | {:?} |", c.key(), c.tier()));
        for u in ConditionUse::ALL {
            assert!(!u.label().is_empty());
            out.push_str(if u.refuses(&c).is_none() {
                " yes |"
            } else {
                " - |"
            });
        }
        out.push('\n');
    }
    println!("{out}");
    // Spot checks.
    assert!(
        ConditionUse::Overheard
            .refuses(&parse(samples[2]))
            .is_none()
    );
    assert!(
        ConditionUse::DilemmaTrigger
            .refuses(&parse(samples[4]))
            .is_some()
    );
    assert!(
        ConditionUse::VisitVerb
            .refuses(&parse(samples[3]))
            .is_none()
    );
}
