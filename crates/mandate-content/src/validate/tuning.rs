//! Tuning and balance-number ranges (REQ-005, DEC-078 decision 1).

use super::*;

/// Tuning numbers in their ranges (REQ-005).
pub(super) fn validate_tuning(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let t = &b.tuning;
    let range = |issues: &mut Vec<Issue>, name: &str, v: u32, lo: u32, hi: u32| {
        if v < lo || v > hi {
            issues.push(Issue::error(
                "tuning",
                &format!("{name} = {v} outside [{lo}, {hi}]"),
            ));
        }
    };
    range(issues, "standard_slots", t.standard_slots.into(), 1, 6);
    range(
        issues,
        "turn_days.campaign",
        t.turn_days.campaign.into(),
        1,
        31,
    );
    range(
        issues,
        "turn_days.final_stretch",
        t.turn_days.final_stretch.into(),
        0,
        14,
    );
    range(
        issues,
        "turn_days.governing_local",
        t.turn_days.governing_local.into(),
        1,
        62,
    );
    range(
        issues,
        "turn_days.governing_regional",
        t.turn_days.governing_regional.into(),
        1,
        62,
    );
    range(
        issues,
        "turn_days.governing_national",
        t.turn_days.governing_national.into(),
        1,
        62,
    );
    range(issues, "burn_in_years", t.burn_in_years.into(), 0, 60);
}

/// Balance numbers in their ranges, and [min, max] pairs ordered.
pub(super) fn validate_param_bounds(b: &ContentBundle, issues: &mut Vec<Issue>) {
    for (name, v, lo, hi) in param_ranges(b) {
        if !(v.is_finite() && (lo..=hi).contains(&v)) {
            let kind = name.split('.').next().unwrap_or("parameters").to_string();
            issues.push(Issue::error(
                &kind,
                &format!("{name} = {v} outside [{lo}, {hi}]"),
            ));
        }
    }
    for (name, [lo, hi]) in [
        ("actions.fundraise.raised", b.actions.fundraise.raised),
        ("council.lobby.gain", b.council.lobby.gain),
    ] {
        if lo > hi {
            issues.push(Issue::error(name, &format!("{name} must be [min, max]")));
        }
    }
    let [lo, hi] = b.npc.bill_sunset_years;
    if lo > hi || lo == 0 {
        issues.push(Issue::error(
            "npc",
            "npc.bill_sunset_years must be [min, max] with min >= 1",
        ));
    }
    if b.actions.town_hall.risk_min > b.actions.town_hall.risk_max
        || b.actions.press.gaffe_min > b.actions.press.gaffe_max
        || b.council.lobby.odds_min > b.council.lobby.odds_max
    {
        issues.push(Issue::error(
            "parameters",
            "risk and odds bounds must be [min, max]",
        ));
    }
}

/// Ranges for the balance numbers (DEC-078 decision 1): (path, value, min,
/// max). Ranges are wide guardrails against typos and broken overrides, not
/// balance targets.
pub fn param_ranges(b: &ContentBundle) -> Vec<(String, f64, f64, f64)> {
    let mut out: Vec<(String, f64, f64, f64)> = Vec::new();
    let mut r = |name: &str, v: f64, lo: f64, hi: f64| out.push((name.to_string(), v, lo, hi));
    let v = &b.politics.vote;
    for (name, x, lo, hi) in [
        ("issue_weight", v.issue_weight, 0.0, 20.0),
        ("beta_low", v.beta_low, 0.0, 1.0),
        ("beta_high", v.beta_high, 0.0, 1.0),
        ("polarization_low", v.polarization_low, 0.0, 2.0),
        ("polarization_high", v.polarization_high, 0.0, 2.0),
        ("party_weight", v.party_weight, 0.0, 20.0),
        ("valence_weight", v.valence_weight, 0.0, 20.0),
        ("retrospective_weight", v.retrospective_weight, 0.0, 50.0),
        ("executive_multiplier", v.executive_multiplier, 0.0, 10.0),
        ("home_ward_bonus", v.home_ward_bonus, 0.0, 5.0),
        ("partisan_discount", v.partisan_discount, 0.0, 1.0),
        ("incumbent_awareness", v.incumbent_awareness, 0.0, 1.0),
        ("challenger_awareness", v.challenger_awareness, 0.0, 1.0),
        ("race_shock_sd", v.race_shock_sd, 0.0, 5.0),
        ("national_swing_sd", v.national_swing_sd, 0.0, 5.0),
        ("strategic_weight", v.strategic_weight, 0.0, 20.0),
        ("minor_party_threshold", v.minor_party_threshold, 0.0, 1.0),
        ("record_penalty", v.record_penalty, 0.0, 50.0),
        ("record_bonus", v.record_bonus, 0.0, 10.0),
        ("record_baseline", v.record_baseline, 0.0, 1.0),
        ("renomination_standing", v.renomination_standing, 0.0, 100.0),
        ("salience_base", v.salience_base, 0.01, 100.0),
        ("perceived_own_weight", v.perceived_own_weight, 0.0, 1.0),
        ("perceived_party_weight", v.perceived_party_weight, 0.0, 1.0),
        ("valence_oratory", v.valence_oratory, 0.0, 5.0),
        ("valence_integrity", v.valence_integrity, 0.0, 5.0),
        ("governing_party_record", v.governing_party_record, 0.0, 1.0),
        ("retro_economy_weight", v.retro_economy_weight, 0.0, 5.0),
        (
            "retro_unemployment_scale",
            v.retro_unemployment_scale,
            0.0,
            100.0,
        ),
        ("retro_level_weight", v.retro_level_weight, 0.0, 5.0),
        ("media_base", v.media_base, 0.0, 5.0),
        ("media_town_share", v.media_town_share, 0.0, 5.0),
        (
            "player_approval_weight",
            v.player_approval_weight,
            0.0,
            20.0,
        ),
        ("independent_viability", v.independent_viability, 0.0, 1.0),
        (
            "player_incumbent_awareness",
            v.player_incumbent_awareness,
            0.0,
            1.0,
        ),
        ("officeholder_awareness", v.officeholder_awareness, 0.0, 1.0),
        ("home_awareness_bonus", v.home_awareness_bonus, 0.0, 1.0),
        ("awareness_cap", v.awareness_cap, 0.0, 1.0),
        ("awareness_known", v.awareness_known, 0.0, 1.0),
        ("awareness_beyond", v.awareness_beyond, 0.0, 1.0),
    ] {
        r(&format!("politics.vote.{name}"), x, lo, hi);
    }
    let pl = &b.politics.party_line;
    for (name, x, lo, hi) in [
        ("platform", pl.platform, 0.0, 5.0),
        ("own_sponsor", pl.own_sponsor, 0.0, 5.0),
        ("governing_sponsor", pl.governing_sponsor, -5.0, 5.0),
        ("other_sponsor", pl.other_sponsor, -5.0, 5.0),
        ("free_band", pl.free_band, 0.0, 2.0),
        ("npc_pull", pl.npc_pull, 0.0, 2.0),
    ] {
        r(&format!("politics.party_line.{name}"), x, lo, hi);
    }
    let t = &b.politics.turnout;
    r("politics.turnout.habit", t.habit, 0.0, 1.0);
    r("politics.turnout.habit_base", t.habit_base, 0.0, 2.0);
    r(
        "politics.turnout.habit_propensity",
        t.habit_propensity,
        0.0,
        2.0,
    );
    r(
        "politics.turnout.mobilisation_max",
        t.mobilisation_max,
        0.0,
        1.0,
    );
    let c = &b.politics.campaign;
    r("politics.campaign.diminishing", c.diminishing, 0.0, 1.0);
    r(
        "politics.campaign.npc_effort_base",
        c.npc_effort_base,
        0.0,
        5.0,
    );
    r(
        "politics.campaign.contact_memory",
        c.contact_memory,
        0.0,
        1.0,
    );
    r("politics.campaign.ad_saturation", c.ad_saturation, 0.0, 1.0);
    r(
        "politics.campaign.ad_persuasion_share",
        c.ad_persuasion_share,
        0.0,
        5.0,
    );
    r(
        "politics.party_id.governing_credit",
        b.politics.party_id.governing_credit,
        0.0,
        2.0,
    );

    let o = &b.opinion;
    let e = &o.experience;
    r(
        "opinion.experience.unemployment",
        e.unemployment,
        0.0,
        100.0,
    );
    r(
        "opinion.experience.unemployment_gap",
        e.unemployment_gap,
        0.0,
        100.0,
    );
    r("opinion.experience.rent_growth", e.rent_growth, 0.0, 100.0);
    r("opinion.experience.services", e.services, 0.0, 100.0);
    r("opinion.experience.disaster", e.disaster, 0.0, 5.0);
    r(
        "opinion.experience.disaster_months",
        e.disaster_months.into(),
        0.0,
        120.0,
    );
    r("opinion.experience.disruption", e.disruption, 0.0, 100.0);
    r(
        "opinion.experience.service_budget",
        e.service_budget,
        0.0,
        100.0,
    );
    r(
        "opinion.experience.delivered.jobs",
        e.delivered.jobs,
        0.0,
        100.0,
    );
    r(
        "opinion.experience.delivered.income",
        e.delivered.income,
        0.0,
        100.0,
    );
    r(
        "opinion.experience.delivered.homes",
        e.delivered.homes,
        0.0,
        1000.0,
    );
    r("opinion.policy_reaction", o.policy_reaction, 0.0, 20.0);
    r(
        "opinion.law_reaction_weight",
        o.law_reaction_weight,
        0.0,
        10.0,
    );
    r("opinion.gov_approval_rate", o.gov_approval_rate, 0.0, 1.0);
    r(
        "opinion.player_approval_rate",
        o.player_approval_rate,
        0.0,
        1.0,
    );
    r(
        "opinion.player_reaction_scale",
        o.player_reaction_scale,
        0.0,
        20.0,
    );
    let lc = &o.local_concerns;
    let kinds = |l: &crate::NeighborhoodLean| {
        [
            ("core", l.core),
            ("inner", l.inner),
            ("waterfront", l.waterfront),
            ("industrial", l.industrial),
            ("suburb", l.suburb),
            ("rural", l.rural),
        ]
    };
    for (k, v) in kinds(&lc.rent) {
        r(&format!("opinion.local_concerns.rent.{k}"), v, 0.05, 10.0);
    }
    for (k, v) in kinds(&lc.pollution) {
        r(
            &format!("opinion.local_concerns.pollution.{k}"),
            v,
            0.0,
            10.0,
        );
    }
    for (name, v, hi) in [
        ("rent_growth", lc.rent_growth, 10.0),
        ("pollution_workplace", lc.pollution_workplace, 10.0),
        ("pollution_next_door", lc.pollution_next_door, 10.0),
        ("pollution_floodplain", lc.pollution_floodplain, 10.0),
        ("workplace_affinity", lc.workplace_affinity, 20.0),
        ("job_loss_decline", lc.job_loss_decline, 50.0),
        ("jobs_sector_tilt", lc.jobs_sector_tilt, 5.0),
    ] {
        r(&format!("opinion.local_concerns.{name}"), v, 0.0, hi);
    }
    let sg = &lc.signals;
    for (name, v, hi) in [
        ("scandal_days", f64::from(sg.scandal_days), 3650.0),
        ("case_open", sg.case_open, 10.0),
        ("case_convicted", sg.case_convicted, 10.0),
        ("care_children", sg.care_children, 10.0),
        ("care_seniors", sg.care_seniors, 10.0),
    ] {
        r(
            &format!("opinion.local_concerns.signals.{name}"),
            v,
            0.0,
            hi,
        );
    }
    for (k, v) in sg.scandal_gravity.iter().enumerate() {
        r(
            &format!("opinion.local_concerns.signals.scandal_gravity.{k}"),
            *v,
            0.0,
            10.0,
        );
    }
    for (k, v) in sg.cost_of_living_class.iter().enumerate() {
        r(
            &format!("opinion.local_concerns.signals.cost_of_living_class.{k}"),
            *v,
            0.0,
            10.0,
        );
    }
    for s in &lc.sector_kinds {
        for (k, v) in kinds(&s.kinds) {
            r(
                &format!("opinion.local_concerns.sector_kinds.{:?}.{k}", s.driver),
                v,
                0.0,
                20.0,
            );
        }
    }

    let a = &b.actions;
    r("actions.late_night_health", a.late_night_health, 0.0, 100.0);
    let vo = &a.vote;
    for (name, x) in [
        ("standing_loyal", vo.standing_loyal),
        ("standing_defy", vo.standing_defy),
        ("standing_abstain", vo.standing_abstain),
    ] {
        r(&format!("actions.vote.{name}"), x, -100.0, 100.0);
    }
    r("actions.vote.ward_reaction", vo.ward_reaction, 0.0, 1.0);
    r("actions.vote.ward_abstain", vo.ward_abstain, -1.0, 1.0);
    for (name, x) in [
        ("sponsor_yes", vo.sponsor_yes),
        ("sponsor_no", vo.sponsor_no),
        ("sponsor_no_owed", vo.sponsor_no_owed),
    ] {
        r(&format!("actions.vote.{name}"), x, 0.0, 100.0);
    }
    r("actions.amend.sway_keep", a.amend.sway_keep, 0.0, 1.0);
    r(
        "actions.introduce.awareness",
        a.introduce.awareness,
        0.0,
        1.0,
    );
    r("actions.lobby.trade_gain", a.lobby.trade_gain, 0.0, 5.0);
    r(
        "actions.lobby.opinion_success",
        a.lobby.opinion_success,
        -100.0,
        100.0,
    );
    r(
        "actions.lobby.opinion_fail",
        a.lobby.opinion_fail,
        -100.0,
        100.0,
    );
    r(
        "actions.lobby.late_night_legal",
        a.lobby.late_night_legal,
        0.0,
        100.0,
    );
    let th = &a.town_hall;
    for (name, x, lo, hi) in [
        ("awareness", th.awareness, 0.0, 1.0),
        (
            "awareness_oratory_base",
            th.awareness_oratory_base,
            0.0,
            5.0,
        ),
        ("bill_nudge", th.bill_nudge, 0.0, 1.0),
        ("nudge", th.nudge, -1.0, 1.0),
        ("hostile_nudge", th.hostile_nudge, -1.0, 1.0),
        ("bill_awareness", th.bill_awareness, 0.0, 1.0),
        ("contact_base", th.contact_base, 0.0, 5.0),
        ("contact_oratory", th.contact_oratory, 0.0, 5.0),
        ("hostile_contact", th.hostile_contact, 0.0, 1.0),
        ("risk_base", th.risk_base, -1.0, 1.0),
        ("risk_approval", th.risk_approval, 0.0, 5.0),
        ("risk_government", th.risk_government, 0.0, 5.0),
        ("risk_min", th.risk_min, 0.0, 1.0),
        ("risk_max", th.risk_max, 0.0, 1.0),
    ] {
        r(&format!("actions.town_hall.{name}"), x, lo, hi);
    }
    let pr = &a.press;
    for (name, x, lo, hi) in [
        ("awareness", pr.awareness, 0.0, 1.0),
        ("gaffe_approval", pr.gaffe_approval, 0.0, 1.0),
        ("bill_awareness", pr.bill_awareness, 0.0, 1.0),
        ("contact", pr.contact, 0.0, 5.0),
        ("gaffe_base", pr.gaffe_base, 0.0, 1.0),
        ("gaffe_oratory", pr.gaffe_oratory, 0.0, 1.0),
        ("gaffe_min", pr.gaffe_min, 0.0, 1.0),
        ("gaffe_max", pr.gaffe_max, 0.0, 1.0),
    ] {
        r(&format!("actions.press.{name}"), x, lo, hi);
    }
    r("actions.canvass.awareness", a.canvass.awareness, 0.0, 1.0);
    r("actions.canvass.approval", a.canvass.approval, 0.0, 1.0);
    r("actions.canvass.contact", a.canvass.contact, 0.0, 10.0);
    let f = &a.fundraise;
    r("actions.fundraise.raised[0]", f.raised[0], 0.0, 1e7);
    r("actions.fundraise.raised[1]", f.raised[1], 0.0, 1e7);
    r("actions.fundraise.standing", f.standing, -100.0, 100.0);
    r(
        "actions.fundraise.late_night_legal",
        f.late_night_legal,
        0.0,
        100.0,
    );
    r("actions.fundraise.donor_ties", f.donor_ties, 0.0, 1.0);

    let co = &b.council;
    for (name, x, lo, hi) in [
        ("own_values", co.own_values, 0.0, 5.0),
        ("ward_support", co.ward_support, 0.0, 5.0),
        ("mayor", co.mayor, 0.0, 5.0),
        ("player_opinion", co.player_opinion, 0.0, 5.0),
        ("sunset", co.sunset, -2.0, 2.0),
        ("pilot", co.pilot, -2.0, 2.0),
        ("foreign_earmark", co.foreign_earmark, -2.0, 2.0),
        ("own_earmark", co.own_earmark, -2.0, 2.0),
        ("noise_sd", co.noise_sd, 0.0, 2.0),
        ("mayor_own", co.mayor_own, 0.0, 5.0),
        ("mayor_public", co.mayor_public, 0.0, 5.0),
        ("vote_sharpness", co.vote_sharpness, 0.0, 100.0),
        ("sign_bias", co.sign_bias, -2.0, 2.0),
        ("enactment_awareness", co.enactment_awareness, 0.0, 1.0),
        ("awareness_fade", co.awareness_fade, 0.0, 1.0),
    ] {
        r(&format!("council.{name}"), x, lo, hi);
    }
    let l = &co.lobby;
    for (name, x, lo, hi) in [
        ("skill_negotiation", l.skill_negotiation, 0.0, 5.0),
        ("skill_oratory", l.skill_oratory, 0.0, 5.0),
        ("skill_scale", l.skill_scale, 0.0, 50.0),
        ("pragmatism", l.pragmatism, 0.0, 50.0),
        ("bias", l.bias, -50.0, 50.0),
        ("opinion_scale", l.opinion_scale, 1.0, 1000.0),
        ("same_party", l.same_party, -10.0, 10.0),
        ("other_party", l.other_party, -10.0, 10.0),
        ("late_night", l.late_night, -10.0, 10.0),
        ("odds_min", l.odds_min, 0.0, 1.0),
        ("odds_max", l.odds_max, 0.0, 1.0),
        ("gain[0]", l.gain[0], 0.0, 5.0),
        ("gain[1]", l.gain[1], 0.0, 5.0),
        ("fail_gain", l.fail_gain, -5.0, 5.0),
    ] {
        r(&format!("council.lobby.{name}"), x, lo, hi);
    }

    let ca = &b.career;
    let g = &ca.record_gain;
    for (name, x) in [
        ("introduce", g.introduce),
        ("amend", g.amend),
        ("vote", g.vote),
        ("lobby", g.lobby),
        ("town_hall", g.town_hall),
        ("canvass", g.canvass),
        ("press", g.press),
    ] {
        r(&format!("career.record_gain.{name}"), x, 0.0, 1.0);
    }
    for (name, x, lo, hi) in [
        ("record_fade", ca.record_fade, 0.0, 1.0),
        ("npc_diligence_rate", ca.npc_diligence_rate, 0.0, 1.0),
        ("diligence_integrity", ca.diligence_integrity, 0.0, 1.0),
        ("diligence_ambition", ca.diligence_ambition, 0.0, 1.0),
        ("thin_record", ca.thin_record, 0.0, 1.0),
        (
            "thin_record_standing",
            ca.thin_record_standing,
            -100.0,
            100.0,
        ),
        ("strong_record", ca.strong_record, 0.0, 1.0),
        (
            "strong_record_standing",
            ca.strong_record_standing,
            -100.0,
            100.0,
        ),
        ("invisible_record", ca.invisible_record, 0.0, 1.0),
        ("awareness_keep", ca.awareness_keep, 0.0, 1.0),
        (
            "awareness_keep_invisible",
            ca.awareness_keep_invisible,
            0.0,
            1.0,
        ),
        ("mandate_rate", ca.mandate_rate, 0.0, 1.0),
        (
            "mandate_per_net_approval",
            ca.mandate_per_net_approval,
            0.0,
            1000.0,
        ),
        ("viability_rate", ca.viability_rate, 0.0, 1.0),
        (
            "viability_standing_share",
            ca.viability_standing_share,
            0.0,
            1.0,
        ),
        ("health_recovery", ca.health_recovery, 0.0, 100.0),
        ("win_mandate_base", ca.win_mandate_base, 0.0, 100.0),
        (
            "win_mandate_per_margin",
            ca.win_mandate_per_margin,
            0.0,
            1000.0,
        ),
        ("win_mandate_min", ca.win_mandate_min, 0.0, 100.0),
        ("win_viability", ca.win_viability, -100.0, 100.0),
        ("win_standing", ca.win_standing, -100.0, 100.0),
        ("loss_viability", ca.loss_viability, -100.0, 100.0),
        (
            "loss_viability_per_margin",
            ca.loss_viability_per_margin,
            0.0,
            1000.0,
        ),
        ("loss_standing", ca.loss_standing, -100.0, 100.0),
        ("scandal_keep_yearly", ca.scandal_keep_yearly, 0.0, 1.0),
        ("officeholder_funds", ca.officeholder_funds, 0.0, 1e7),
        (
            "officeholder_funds_oratory",
            ca.officeholder_funds_oratory,
            0.0,
            1e7,
        ),
    ] {
        r(&format!("career.{name}"), x, lo, hi);
    }

    let n = &b.npc;
    for (name, x, lo, hi) in [
        ("ideologue_below", n.ideologue_below, 0.0, 1.0),
        ("populist_above", n.populist_above, 0.0, 1.0),
        ("close_margin", n.close_margin, 0.01, 1.0),
        ("funds_in_ads", n.funds_in_ads, 0.0, 100.0),
        ("busy_pending", n.busy_pending, 0.1, 100.0),
        ("busy_laws", n.busy_laws, 0.1, 100.0),
        (
            "bill_interval_days",
            n.bill_interval_days.into(),
            0.0,
            3650.0,
        ),
        ("insurance_votes", n.insurance_votes, 0.0, 10.0),
        ("safe_gap", n.safe_gap, 0.01, 100.0),
        ("comfortable_weight", n.comfortable_weight, 0.0, 1.0),
        ("lobby_closeness_keep", n.lobby_closeness_keep, 0.0, 1.0),
        ("contact_unknown_keep", n.contact_unknown_keep, 0.0, 1.0),
        ("two_ads_above", n.two_ads_above, 0.0, 10.0),
        ("one_ad_above", n.one_ad_above, 0.0, 10.0),
        ("ad_reserve", n.ad_reserve, 0.0, 100.0),
    ] {
        r(&format!("npc.{name}"), x, lo, hi);
    }
    r("npc.crisis_agenda", n.crisis_agenda, 0.0, 5.0);
    r("npc.concern_agenda", n.concern_agenda, 0.0, 5.0);

    let ch = &b.characters;
    r(
        "characters.memory.principal",
        ch.memory.principal.into(),
        1.0,
        100.0,
    );
    r(
        "characters.memory.named",
        ch.memory.named.into(),
        1.0,
        100.0,
    );
    r(
        "characters.memory.forget_below",
        ch.memory.forget_below,
        0.0,
        1.0,
    );
    for (k, m) in &ch.memories {
        let p = format!("characters.memories.{k}");
        r(&format!("{p}.sentiment"), m.sentiment, -1.0, 1.0);
        r(&format!("{p}.magnitude"), m.magnitude, 0.0, 1.0);
        r(&format!("{p}.decay"), m.decay, 0.0, 20.0);
        r(&format!("{p}.opinion"), m.opinion, -100.0, 100.0);
        r(&format!("{p}.trust"), m.trust, -1.0, 1.0);
    }
    let g = &ch.generation;
    for (name, [lo, hi]) in [("personality", g.personality), ("competency", g.competency)] {
        r(&format!("characters.generation.{name}[0]"), lo, 0.0, 1.0);
        r(&format!("characters.generation.{name}[1]"), hi, lo, 1.0);
    }
    r("characters.generation.values_sd", g.values_sd, 0.0, 1.0);
    r(
        "characters.generation.first_impression_sd",
        g.first_impression_sd,
        0.0,
        100.0,
    );
    r(
        "characters.generation.family_chance",
        g.family_chance,
        0.0,
        1.0,
    );
    r("characters.generation.drift_sd", g.drift_sd, 0.0, 1.0);
    r(
        "characters.generation.trust_initial",
        g.trust_initial,
        0.0,
        1.0,
    );
    r(
        "characters.bonds.ally_above",
        ch.bonds.ally_above,
        -100.0,
        100.0,
    );
    r(
        "characters.bonds.rival_below",
        ch.bonds.rival_below,
        -100.0,
        ch.bonds.ally_above,
    );
    r(
        "council.loyalty_discipline",
        b.council.loyalty_discipline,
        0.0,
        5.0,
    );
    r(
        "council.lobby.integrity",
        b.council.lobby.integrity,
        0.0,
        10.0,
    );
    r(
        "characters.ai.warmth_opinion",
        ch.ai.warmth_opinion,
        0.0,
        1.0,
    );
    let a = &ch.ambient;
    r(
        "characters.ambient.present[0]",
        a.present[0].into(),
        0.0,
        10.0,
    );
    r(
        "characters.ambient.present[1]",
        a.present[1].into(),
        a.present[0].into(),
        10.0,
    );
    r(
        "characters.ambient.period_days",
        a.period_days.into(),
        1.0,
        366.0,
    );
    r(
        "characters.ambient.max_listed",
        a.max_listed.into(),
        1.0,
        20.0,
    );
    r(
        "characters.town.trend_months",
        ch.town.trend_months.into(),
        1.0,
        36.0,
    );
    r(
        "characters.town.broker_seniority",
        ch.town.broker_seniority,
        0.0,
        10.0,
    );
    r(
        "characters.town.broker_stake",
        ch.town.broker_stake,
        0.0,
        10.0,
    );
    // The power broker (M6 note 27): signed amounts within their sign.
    let k = &ch.broker;
    for (name, v, lo, hi) in [
        ("vote_pull", k.vote_pull, 0.0, 2.0),
        ("officeholder_influence", k.officeholder_influence, 0.0, 1.0),
        ("back_money", k.back_money, 0.0, 1.0e6),
        ("back_contacts", k.back_contacts.into(), 0.0, 20.0),
        ("back_values", k.back_values, 0.0, 100.0),
        ("back_min_opinion", k.back_min_opinion, -100.0, 100.0),
        ("lobby_sway", k.lobby_sway, 0.0, 2.0),
        ("lobby_min_opinion", k.lobby_min_opinion, -100.0, 100.0),
        ("favour_opinion", k.favour_opinion, 0.0, 50.0),
        (
            "favour_resent_approval",
            k.favour_resent_approval,
            -0.5,
            0.0,
        ),
        ("oppose_resent_approval", k.oppose_resent_approval, 0.0, 0.5),
        (
            "oppose_depend_approval",
            k.oppose_depend_approval,
            -0.5,
            0.0,
        ),
        ("oppose_opinion", k.oppose_opinion, -100.0, 0.0),
        ("oppose_ally_opinion", k.oppose_ally_opinion, -50.0, 0.0),
        (
            "expose_member_opinion",
            k.expose_member_opinion,
            -100.0,
            0.0,
        ),
        ("expose_resent_approval", k.expose_resent_approval, 0.0, 0.5),
        ("expose_opinion", k.expose_opinion, -100.0, 0.0),
        ("expose_gaffe_approval", k.expose_gaffe_approval, -0.5, 0.0),
        ("cooldown_days", k.cooldown_days.into(), 1.0, 3650.0),
    ] {
        r(&format!("characters.broker.{name}"), v, lo, hi);
    }
    crate::narrative::director_ranges(&b.director, &mut out);
    crate::media::param_ranges(b, &mut |n, v, lo, hi| out.push((n.to_string(), v, lo, hi)));
    crate::knowledge::param_ranges(b, &mut |n, v, lo, hi| out.push((n.to_string(), v, lo, hi)));
    crate::conversation::param_ranges(b, &mut |n, v, lo, hi| out.push((n.to_string(), v, lo, hi)));
    out
}
