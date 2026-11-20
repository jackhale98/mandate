#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Ask and promise validation (E6, DEC-097 §2; ENGAGEMENT §3.6): every ask
//! gains when kept and costs when broken, needs name roles of the right kind,
//! deadlines stay within [7, 900], triggers never read what the player knows,
//! Fluent keys exist with the arguments the ask provides, promises name what
//! they answer, and the intent curves are well formed.
use mandate_content::{Severity, Source, bundle::read_dir_sources, load};
use std::path::Path;

fn with(path: &str, extra: &str) -> Vec<String> {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(Source {
        path: path.into(),
        text: extra.into(),
    });
    match load(&src) {
        Ok(_) => Vec::new(),
        Err(issues) => issues
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.to_string())
            .collect(),
    }
}

/// An ask in a test pack, its fields spliced in.
fn ask(fields: &str) -> String {
    format!(
        r#"{{"kind":"asks","items":[{{"id":"ask.test","name_key":"ask-casework",
        "asker":{{"type":"occupation","occupations":["occupation.resident"]}},
        "detail_key":"ask-casework-detail","need_key":"ask-casework-need",
        "kept_key":"ask-casework-kept","broken_key":"ask-casework-broken",
        "cooldown_days":30,{fields}}}]}}"#
    )
}

const KEEP: &str = r#""keep":[{"type":"opinion","role":"asker","delta":5}]"#;
const BREAK: &str = r#""break":[{"type":"opinion","role":"asker","delta":-5}]"#;

fn errs(fields: &str) -> Vec<String> {
    with("aa_test/asks.jsonc", &ask(fields))
}

#[test]
fn a_well_formed_ask_loads() {
    let e = errs(&format!(
        r#""needs":[{{"type":"casework"}}],"deadline_days":[21,35],{KEEP},{BREAK}"#
    ));
    assert!(e.is_empty(), "{e:?}");
}

#[test]
fn asks_never_read_what_the_player_knows() {
    let e = errs(&format!(
        r#""triggers":[{{"type":"knows","subject":{{"type":"broker"}}}}],
        "needs":[{{"type":"casework"}}],"deadline_days":[21,35],{KEEP},{BREAK}"#
    ));
    assert!(e.iter().any(|x| x.contains("knows")), "{e:?}");
}

#[test]
fn every_ask_gains_when_kept_and_costs_when_broken() {
    let e = errs(&format!(
        r#""needs":[{{"type":"casework"}}],"deadline_days":[21,35],
        "keep":[{{"type":"opinion","role":"asker","delta":-1}}],{BREAK}"#
    ));
    assert!(e.iter().any(|x| x.contains("needs a gain")), "{e:?}");
    let e = errs(&format!(
        r#""needs":[{{"type":"casework"}}],"deadline_days":[21,35],{KEEP},
        "break":[{{"type":"opinion","role":"asker","delta":2}}]"#
    ));
    assert!(e.iter().any(|x| x.contains("needs a cost")), "{e:?}");
    // Effects the asks system cannot request are refused.
    let e = errs(&format!(
        r#""needs":[{{"type":"casework"}}],"deadline_days":[21,35],{BREAK},
        "keep":[{{"type":"opinion","role":"asker","delta":5}},{{"type":"slots","n":1}}]"#
    ));
    assert!(e.iter().any(|x| x.contains("ask effects are")), "{e:?}");
}

#[test]
fn deadlines_and_needs_are_checked() {
    let e = errs(&format!(
        r#""needs":[{{"type":"casework"}}],"deadline_days":[3,950],{KEEP},{BREAK}"#
    ));
    assert!(e.iter().any(|x| x.contains("deadline_days")), "{e:?}");
    // Without a deadline, an ask is due by a vote.
    let e = errs(&format!(r#""needs":[{{"type":"casework"}}],{KEEP},{BREAK}"#));
    assert!(e.iter().any(|x| x.contains("due by a vote")), "{e:?}");
    // A vote on a role that is not a bill.
    let e = errs(&format!(
        r#""needs":[{{"type":"vote","role":"asker","vote":"yes"}}],{KEEP},{BREAK}"#
    ));
    assert!(e.iter().any(|x| x.contains("names a bill role")), "{e:?}");
    // An unknown role.
    let e = errs(&format!(
        r#""needs":[{{"type":"talk","role":"ghost"}}],"deadline_days":[21,35],{KEEP},{BREAK}"#
    ));
    assert!(e.iter().any(|x| x.contains("unknown role `ghost`")), "{e:?}");
    // A need kept by not acting stands alone.
    let e = errs(&format!(
        r#""needs":[{{"type":"no_surfaced_hidden_fact"}},{{"type":"casework"}}],"deadline_days":[21,35],{KEEP},{BREAK}"#
    ));
    assert!(e.iter().any(|x| x.contains("stands alone")), "{e:?}");
}

#[test]
fn fluent_keys_exist_with_the_arguments_provided() {
    let e = with(
        "aa_test/asks.jsonc",
        &ask(&format!(
            r#""needs":[{{"type":"casework"}}],"deadline_days":[21,35],{KEEP},{BREAK}"#
        ))
        .replace("ask-casework-need", "ask-no-such-key"),
    );
    assert!(e.iter().any(|x| x.contains("missing Fluent key")), "{e:?}");
    // `ask-vote-with-me-need` uses `$bill`, which this ask has no role for.
    let e = with(
        "aa_test/asks.jsonc",
        &ask(&format!(
            r#""needs":[{{"type":"casework"}}],"deadline_days":[21,35],{KEEP},{BREAK}"#
        ))
        .replace("ask-casework-need", "ask-vote-with-me-need"),
    );
    assert!(e.iter().any(|x| x.contains("$bill")), "{e:?}");
}

#[test]
fn promises_name_what_they_answer() {
    let p = r#"{"kind":"promises","items":[{"id":"promise.test","name_key":"promise-jobs",
        "answers":{"type":"concern","concern":"concern.nothing"},
        "needs":[{"type":"town_hall","constituency":true}],"due":{"type":"days","days":180},
        "keep":[{"type":"approval","scope":"ward","delta":0.02}],
        "break":[{"type":"approval","scope":"ward","delta":-0.03}],
        "detail_key":"promise-jobs-detail","need_key":"promise-jobs-need",
        "kept_key":"promise-jobs-kept","broken_key":"promise-jobs-broken"}]}"#;
    // The concern registry is moddable: a warning, as in conditions.
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(Source {
        path: "aa_test/promises.jsonc".into(),
        text: p.into(),
    });
    let (_, warnings) = load(&src).unwrap();
    assert!(
        warnings.iter().any(|x| x.to_string().contains("unknown concern")),
        "{warnings:?}"
    );
}

#[test]
fn the_intent_curves_are_well_formed() {
    let params = |open: &str| {
        format!(
            r#"{{"kind":"ask_params","value":{{"max_open":4,"new_per_month":1.5,"new_per_month_campaign":2.5,
            "intent_by_open":{open},"intent_by_quiet":[[0,0.0],[21,1.0]],"respond_days":14,
            "per_asker_cooldown_days":60,"asker_kept_mult":1.5,"asker_broken_mult":0.5,
            "doorstep_chance":0.35,"history":60,"deeds_kept":120,
            "casework":{{"success_in_office":0.85,"success_out_of_office":0.5}},
            "promises":{{"max_per_campaign":3,"persuasion":0.04,"any_salience":0.5,"broken_discount":0.5}}}}}}"#
        )
    };
    // Defined twice (the base pack has one): refused, which also shows the
    // kind loads.
    let e = with("aa_test/params.jsonc", &params("[[0,2.0],[4,0.0]]"));
    assert!(e.iter().any(|x| x.contains("ask_params defined twice")), "{e:?}");
    // On its own, a curve that is not 0 at max_open, and one that goes back.
    let mut src: Vec<Source> =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
            .unwrap()
            .into_iter()
            .filter(|s| !s.path.ends_with("asks/params.jsonc"))
            .collect();
    src.push(Source {
        path: "base/asks/params.jsonc".into(),
        text: params("[[0,2.0],[2,1.0],[1,0.5]]"),
    });
    let e: Vec<String> = match load(&src) {
        Ok(_) => Vec::new(),
        Err(i) => i.into_iter().map(|i| i.to_string()).collect(),
    };
    assert!(e.iter().any(|x| x.contains("x must increase")), "{e:?}");
    assert!(e.iter().any(|x| x.contains("0 at max_open")), "{e:?}");
}
