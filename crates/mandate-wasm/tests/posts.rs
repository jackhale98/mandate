//! Constitution posts in the protocol (DEC-098 §1, R2): the views describe
//! bodies, offices and posts from the constitution's lists and labels; the
//! `File` command takes a post the campaign view offered; jurisdictions of
//! every tier. Under a constitution whose local posts are renamed, nothing
//! in these views names the base constitution's posts.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::politics::Post;
use mandate_sim::politics::elections::Filing;
use mandate_sim::world::Jurisdiction;
use mandate_sim::{Command, Engine};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::{Arc, OnceLock};

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        Arc::new(
            mandate_content::load(&mandate_content::embedded::sources())
                .unwrap()
                .0,
        )
    })
    .clone()
}

/// The base pack with its local posts renamed (as in the save crate's and
/// the visits tests): the council is a `moot` of `alderman`s, the mayor a
/// `reeve`, with Fluent labels of their own.
fn renamed() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        let mut src = mandate_content::embedded::sources();
        for s in src.iter_mut() {
            let p = s.path.replace('\\', "/");
            if p.contains("locale/") {
                if p.ends_with("base.ftl") {
                    s.text.push_str(
                        "\nbody-moot = Town moot\noffice-reeve = Reeve\nrole-alderman = Alderman\n\
                         role-reeve = Reeve\nstage-vote-moot = Moot vote\nstage-sign-reeve = Reeve's seal\n\
                         outcome-filed-moot = Filed for the moot.\noutcome-filed-reeve = Filed for reeve.\n\
                         talk-body-moot = the moot\ntalk-body-moot-member = an alderman\ntalk-office-reeve = the reeve\n",
                    );
                }
                continue;
            }
            if p.ends_with("council.jsonc") || p.ends_with("interiors.jsonc") {
                continue;
            }
            let mut t = s
                .text
                .replace(r#"{ "type": "mayor" }"#, r#"{ "type": "office" }"#);
            t = t.replace("\"councillor\"", "\"alderman\"");
            t = t.replace("\"mayor\"", "\"reeve\"");
            if p.contains("constitutions/") {
                t = t.replace("\"council\"", "\"moot\"");
                t = t.replace("\"body-council\"", "\"body-moot\"");
                t = t.replace("\"office-mayor\"", "\"office-reeve\"");
            }
            s.text = t;
        }
        Arc::new(mandate_content::load(&src).unwrap().0)
    })
    .clone()
}

fn game(seed: u64, c: Arc<ContentBundle>) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c).unwrap()
}

/// File through the routed command (it walks to where filing is done).
fn file(e: &mut Engine, filing: Filing) {
    let cmd = e.route(PlayerCommand::File { filing });
    e.apply(cmd).unwrap().result.unwrap();
}

/// Draft a standard bill and put it before the legislature.
fn bill(e: &mut Engine) -> u32 {
    let template = e
        .content()
        .policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none())
        .unwrap()
        .id
        .clone();
    let cmd = e.route(PlayerCommand::DraftPolicy {
        template,
        params: Default::default(),
    });
    let id = e.apply(cmd).unwrap().result.unwrap().created.unwrap();
    let cmd = e.route(PlayerCommand::IntroducePolicy { policy: id });
    e.apply(cmd).unwrap().result.unwrap();
    id
}

/// Every label and id a view carries for posts and bodies, for the naming
/// check.
fn words(o: &OfficeView, c: &CampaignView) -> Vec<String> {
    let mut w = Vec::new();
    let post = |w: &mut Vec<String>, p: &PostDto| {
        w.extend([p.id.clone(), p.label_key.clone(), p.role_key.clone()]);
    };
    for p in &o.player.posts {
        post(&mut w, p);
    }
    w.push(o.player.role_key.clone());
    for b in &o.bodies {
        w.extend([b.id.clone(), b.label_key.clone(), b.role_key.clone()]);
        w.extend(b.members.iter().map(|m| m.role_key.clone()));
    }
    w.extend(o.executive.iter().map(|m| m.role_key.clone()));
    for b in &o.bills {
        if let Some(s) = &b.signer {
            post(&mut w, &s.office);
        }
        w.extend(b.next_stage_key.clone());
    }
    for n in &c.nominations {
        post(&mut w, &n.post);
    }
    for r in &c.races {
        post(&mut w, &r.post);
    }
    w
}

#[test]
fn views_describe_posts_from_the_constitution() {
    for (c, body, office) in [
        (content(), "council", "mayor"),
        (renamed(), "moot", "reeve"),
    ] {
        let mut e = game(4040, c.clone());
        let id = bill(&mut e);
        let o = office_view(&e).unwrap();
        // The player sits in the town's legislature: one post, a seat.
        assert_eq!(o.player.posts.len(), 1);
        let me = &o.player.posts[0];
        assert_eq!(me.kind, PostKindDto::Seat);
        assert_eq!(me.id, body);
        assert_eq!(me.label_key, format!("body-{body}"));
        assert_eq!(me.at.kind, JurisdictionKindDto::Municipality);
        assert_eq!(me.at.name, o.player.municipality);
        assert_eq!(me.constituency_number, Some(o.player.ward_number));
        assert_eq!(o.player.role_key, me.role_key);
        // Bodies and the executive, and the aliases that mirror them.
        assert_eq!(o.bodies.len(), 1);
        let b = &o.bodies[0];
        assert!(b.legislature);
        assert_eq!(b.id, body);
        assert!(b.members.iter().any(|m| m.is_player));
        assert_eq!(o.council, b.members);
        assert_eq!(o.mayor, o.executive);
        assert_eq!(
            o.executive.as_ref().map(|m| m.role_key.as_str()),
            Some(format!("role-{office}").as_str())
        );
        // The bill's signer is the executive office of its town.
        let bv = o.bills.iter().find(|x| x.id == id).unwrap();
        let s = bv.signer.as_ref().unwrap();
        assert_eq!(
            (s.office.kind, s.office.id.as_str()),
            (PostKindDto::Office, office)
        );
        assert_eq!(s.office.label_key, format!("office-{office}"));
        assert!(["sign", "lean_sign", "undecided", "lean_veto", "veto"].contains(&s.lean.as_str()));
        // Nominations: the seat held, then the executive office.
        let cv = campaign_view(&e).unwrap();
        let ids: Vec<(&str, bool, bool)> = cv
            .nominations
            .iter()
            .map(|n| (n.post.id.as_str(), n.held, n.chosen))
            .collect();
        assert_eq!(ids, vec![(body, true, true), (office, false, false)]);
        assert_eq!(cv.nominations[0].post, *me);
        assert!(cv.filing.is_none());
        // Nothing names the base constitution's posts under the renamed one.
        if body == "moot" {
            for w in words(&o, &cv) {
                assert!(!w.contains("council") && !w.contains("mayor"), "{w}");
            }
        }
    }
}

#[test]
fn filing_for_the_executive_office_names_its_race_and_rival_plan() {
    // A town whose executive post is open to file for (tests/seeds.json, found
    // again when the world changes).
    let seed = mandate_seeds::seed("executive_post_open_to_file_for");
    for c in [content(), renamed()] {
        let mut e = game(seed, c.clone());
        let cv = campaign_view(&e).unwrap();
        let exec = cv
            .nominations
            .iter()
            .find(|n| n.post.kind == PostKindDto::Office)
            .unwrap();
        assert!(exec.open && !exec.held);
        let post = campaign::fileable(&e, &exec.post).unwrap();
        assert!(post.is_office());
        file(&mut e, Filing::Stand { post });
        let cv = campaign_view(&e).unwrap();
        assert_eq!(
            cv.filing,
            Some(FilingView::Stand {
                post: exec.post.clone()
            })
        );
        // The rival means to contest that post: labelled by it.
        e.set_lab_omniscient(true);
        let plan = town_places_view(&e)
            .unwrap()
            .character
            .unwrap()
            .rival_plan
            .unwrap();
        assert_eq!(plan.post.as_ref(), Some(&exec.post));
        // The campaign opens: the player's race is for that post.
        for _ in 0..30 {
            if !campaign_view(&e).unwrap().races.is_empty() {
                break;
            }
            e.apply(Command::EndTurn).unwrap();
        }
        let cv = campaign_view(&e).unwrap();
        let mine = cv.races.iter().find(|r| r.is_player_race).unwrap();
        assert_eq!(mine.post, exec.post);
        // Offices first, then seats by district.
        assert!(cv.races.iter().all(|r| r.post.at == mine.post.at));
        assert!(
            cv.races
                .iter()
                .skip(1)
                .any(|r| r.post.kind == PostKindDto::Seat)
        );
    }
}

/// A filing made on the first turn still stands a month on: the first
/// monthly tick lands the history's last election on the career, but only
/// an election held during play ends a filing (`player::monthly`).
#[test]
fn a_filing_made_on_the_first_turn_holds_a_month_later() {
    let mut e = game(1, content());
    let st = e.state();
    let last = st.politics.elections.last_election.unwrap();
    assert!(last < st.calendar.date, "the history's election");
    assert_eq!(st.player.as_ref().unwrap().last_election_seen, None);
    let exec = campaign_view(&e)
        .unwrap()
        .nominations
        .into_iter()
        .find(|n| n.post.kind == PostKindDto::Office)
        .unwrap();
    let post = campaign::fileable(&e, &exec.post).unwrap();
    file(&mut e, Filing::Stand { post });
    let start = e.state().calendar.date;
    while e.state().calendar.date.0 < start.0 + 40 {
        e.apply(Command::EndTurn).unwrap();
    }
    // The history's election has landed, and the filing stands.
    assert_eq!(
        e.state().player.as_ref().unwrap().last_election_seen,
        Some(last)
    );
    assert_eq!(
        e.state().player.as_ref().unwrap().filing,
        Some(Filing::Stand { post })
    );
    assert_eq!(
        campaign_view(&e).unwrap().filing,
        Some(FilingView::Stand { post: exec.post })
    );
}

/// A request and its response, each through JSON (as the worker sends them).
fn send(s: &mut Session, body: RequestBody) -> Result<ResponseBody, ProtocolError> {
    let json = serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: 1,
        body,
    })
    .unwrap();
    let res = s.handle(serde_json::from_str(&json).unwrap());
    let back: Response = serde_json::from_str(&serde_json::to_string(&res).unwrap()).unwrap();
    match back.result {
        ResponseResult::Ok { body } => Ok(body),
        ResponseResult::Err { error } => Err(error),
    }
}

fn campaign_of(s: &mut Session) -> CampaignView {
    match send(s, RequestBody::Query(QueryDto::Campaign)).unwrap() {
        ResponseBody::Campaign(c) => c,
        other => panic!("{other:?}"),
    }
}

fn filing_of(s: &Session) -> Option<Filing> {
    s.engine().unwrap().state().player.as_ref().unwrap().filing
}

/// The client's posts go back to the engine as JSON (the session's `File`
/// command): the offered post, retiring, or (with neither) the post held;
/// anything else is refused.
#[test]
fn the_file_command_round_trips_through_json() {
    let mut s = Session::new().unwrap();
    send(
        &mut s,
        RequestBody::NewGame(NewGameParams {
            seed: 4040,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
            recent_openings: Vec::new(),
        }),
    )
    .unwrap();
    let cv = campaign_of(&mut s);
    let held = cv.nominations.iter().find(|n| n.held).unwrap().post.clone();
    let exec = cv
        .nominations
        .iter()
        .find(|n| n.post.kind == PostKindDto::Office)
        .unwrap()
        .post
        .clone();
    let town = match send(&mut s, RequestBody::Query(QueryDto::Town)).unwrap() {
        ResponseBody::Town(t) => t,
        other => panic!("{other:?}"),
    };
    let at = town
        .neighborhoods
        .iter()
        .flat_map(|h| &h.spots)
        .find(|sp| sp.actions.iter().any(|a| a == "file"))
        .unwrap()
        .location
        .location;
    let file = |s: &mut Session, post: Option<PostDto>, retire: bool| {
        send(
            s,
            RequestBody::Command(CommandDto::Go {
                location: at,
                then: Some(Box::new(CommandDto::File { post, retire })),
            }),
        )
    };
    file(&mut s, Some(exec.clone()), false).unwrap();
    assert!(matches!(filing_of(&s), Some(Filing::Stand { post }) if post.is_office()));
    assert_eq!(
        campaign_of(&mut s).filing,
        Some(FilingView::Stand { post: exec.clone() })
    );
    file(&mut s, None, true).unwrap();
    assert_eq!(filing_of(&s), Some(Filing::Retire));
    file(&mut s, None, false).unwrap();
    let held_post = filing_of(&s);
    assert!(matches!(held_post, Some(Filing::Stand { post }) if !post.is_office()));
    assert_eq!(
        campaign_of(&mut s).filing,
        Some(FilingView::Stand { post: held.clone() })
    );
    // Not a post the player may stand for: refused, the filing unchanged.
    let mut bogus = exec.clone();
    bogus.id = "no_such_office".into();
    assert_eq!(
        file(&mut s, Some(bogus), false).unwrap_err().code,
        ErrorCode::InvalidCommand
    );
    let mut other_ward = held.clone();
    other_ward.constituency_number = other_ward.constituency_number.map(|n| n % 9 + 1);
    assert!(file(&mut s, Some(other_ward), false).is_err());
    assert_eq!(filing_of(&s), held_post);
}

/// Jurisdictions and posts of every tier, both ways (M7 fills the higher
/// tiers; F3's proposals carry `JurisdictionDto`).
#[test]
fn jurisdictions_and_posts_of_every_tier_round_trip() {
    let e = game(4040, content());
    let st = e.state();
    let town = Jurisdiction::town(mandate_sim::player::home(st).unwrap());
    let region = town.parent(&st.world).unwrap();
    for (at, kind, tier) in [
        (town, JurisdictionKindDto::Municipality, "local"),
        (region, JurisdictionKindDto::Region, "regional"),
        (
            Jurisdiction::Nation,
            JurisdictionKindDto::Nation,
            "national",
        ),
    ] {
        let dto = jurisdiction_dto(&st.world, at);
        assert_eq!((dto.kind, dto.tier.as_str()), (kind, tier));
        assert_eq!(
            dto.name_key,
            format!(
                "jurisdiction-{}",
                serde_json::to_value(kind).unwrap().as_str().unwrap()
            )
        );
        assert_eq!(dto.name.is_empty(), kind == JurisdictionKindDto::Nation);
        let json = serde_json::to_string(&dto).unwrap();
        let back: JurisdictionDto = serde_json::from_str(&json).unwrap();
        assert_eq!(jurisdiction_of(&st.world, &back), Some(at));
    }
    let key = |s: &str| mandate_content::PostKey::new(s).unwrap();
    let ward = region_constituency(&e, region);
    for p in [
        Post::Office {
            office: key("president"),
            at: Jurisdiction::Nation,
        },
        Post::Seat {
            body: key("senate"),
            at: Jurisdiction::Nation,
            constituency: None,
        },
        Post::Seat {
            body: key("assembly"),
            at: region,
            constituency: ward,
        },
    ] {
        let dto = post_dto(&e, p);
        assert!(
            !dto.label_key.is_empty() && dto.role_key.starts_with("role-"),
            "{dto:?}"
        );
        let back: PostDto = serde_json::from_str(&serde_json::to_string(&dto).unwrap()).unwrap();
        assert_eq!(post_of(&e, &back), Some(p));
    }
    // An office named on the wrong tier is not a post.
    let mut wrong = post_dto(
        &e,
        Post::Office {
            office: key("president"),
            at: Jurisdiction::Nation,
        },
    );
    wrong.at = jurisdiction_dto(&st.world, town);
    assert_eq!(post_of(&e, &wrong), None);
}

fn region_constituency(e: &Engine, at: Jurisdiction) -> Option<mandate_sim::world::ConstituencyId> {
    at.constituencies(&e.state().world).next().map(|c| c.id)
}
