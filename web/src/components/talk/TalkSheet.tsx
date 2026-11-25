// Talking to someone (DEC-093), in dialogue mode: a classic RPG dialogue box
// at the bottom of the screen and speech bubbles over the speakers.
//   - In the street (someone standing there by the player, ./stage): the
//     camera frames the two of them facing each other, the rest of the
//     street dims and walking pauses; bubbles follow their sprites.
//   - Anywhere else (the People tab, a place's people, the List view, anyone
//     not on screen): the same box over a large pixel portrait of the person
//     (../portrait), bubbles beside it.
// The box lists the topics the engine offers (`TalkView`) as one-tap
// choices with the engine's odds (DEC-073); a topic about something opens a
// second step in the same box with its subjects. The person speaks first:
// their greeting (the view's) is in their bubble as the box opens, with the
// choices beside it (M6 note 19). Then the player's line goes up over them,
// and the reply (`ExchangeView`) follows in the person's bubble, page by
// page (tap, Enter or Space), stage directions in italics; what was learned
// becomes a notebook toast and a lead a choice ("Go to the market"). The UI
// decides nothing: it sends the `Talk` command and renders what the engine
// says (DEC-054).
// Accessibility (DEC-051): the box is DOM; choices are buttons in a list
// (arrow keys move, Enter chooses, Escape steps back or leaves); the reply
// is read out by a polite live region; focus goes back to the street or to
// whatever opened the conversation.
import { Fragment, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import type { AboutChoiceView } from "../../engine/protocol/AboutChoiceView";
import type { ExchangeView } from "../../engine/protocol/ExchangeView";
import type { LeadView } from "../../engine/protocol/LeadView";
import type { TalkView } from "../../engine/protocol/TalkView";
import type { TopicView } from "../../engine/protocol/TopicView";
import { useGame } from "../../state/gameStore";
import { topicLabel } from "../asks/choiceText";
import { placeName } from "../../places/labels";
import { roleText } from "../CharacterSheet";
import { Portrait } from "../portrait/Portrait";
import { keepLook, lookVariant } from "../portrait/look";
import { Bubble, placeBubble } from "./Bubble";
import { costText, greetingPage, pageText, questionText, replyPages, SEARCH_FROM, subjectGroups, subjectSplit, topicRow, type Page } from "./dialogueModel";
import { setTalkMode, useTalkStage, type TalkMode } from "./stage";
import { useTalkFocus } from "./focus";
import { displayName } from "./streetPeople";
import { aboutText, learnedText } from "./talkText";
import { SceneSheet } from "../exchange/SceneSheet";
import "./talk.css";

const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The typewriter: characters a tick, and the tick (ms). */
const TYPE_STEP = 2;
const TYPE_MS = 28;
/** The player's line stays up at least this long (from when they ask) before the reply replaces it (ms). */
const ASK_HOLD_MS = 900;
/** The notebook toast goes after this long (ms) unless the pointer or the keyboard is on it. */
const NOTED_MS = 8000;

/** One conversation: with whom (their key now), where it is shown, and the look the portrait draws. */
interface Conversation {
  n: number;
  key: string;
  mode: TalkMode;
  look: number;
}

const targetKey = (to: { type: "person"; person: number } | { type: "ambient"; slot: number } | null) => (to ? (to.type === "person" ? `c${to.person}` : `a${to.slot}`) : null);

/**
 * The conversation view: mounted by the game screen. A conversation while the
 * engine offers a talk view; a scene of an arc (DEC-112) while one is open,
 * played in the same box (../exchange/SceneSheet).
 */
export function TalkSheet() {
  return (
    <>
      <Conversation />
      <SceneSheet />
    </>
  );
}

/** A conversation, shown while the engine offers a talk view. */
function Conversation() {
  const v = useGame((s) => s.talk);
  const to = useGame((s) => s.talkTo);
  const x = useGame((s) => s.exchange);
  const [conv, setConv] = useState<Conversation | null>(null);
  const key = targetKey(to);
  if (key !== (conv?.key ?? null)) {
    if (key === null) setConv(null);
    else if (conv && conv.key.startsWith("a") && key.startsWith("c") && x?.speaker.id === Number(key.slice(1))) {
      // A passer-by met as the conversation started: the same conversation, with them as someone known.
      setConv({ ...conv, key });
    } else {
      const stage = useTalkStage.getState().stage;
      setConv({ n: (conv?.n ?? 0) + 1, key, mode: stage?.canFrame(key) ? "street" : "portrait", look: stage?.lookOf(key) ?? lookVariant(key) });
    }
  }
  // The street pauses behind a portrait (nobody sees it).
  const mode = v && conv ? conv.mode : null;
  useEffect(() => {
    setTalkMode(mode);
  }, [mode]);
  useEffect(() => () => setTalkMode(null), []);
  // The portrait keeps a passer-by's look once they are met.
  useEffect(() => {
    if (conv) keepLook(conv.key, conv.look);
  }, [conv]);
  const lost = useCallback(() => setConv((c) => (c ? { ...c, mode: "portrait" } : c)), []);
  if (!v || !conv) return null;
  return <Dialogue key={conv.n} v={v} x={x} conv={conv} onLost={lost} />;
}

// The subject step lists the subjects the engine orders first (`likely`:
// what the person could be expected to know, from public facts); the rest
// wait behind "Something else…" (`more`), with search (M6 note 26).
// Lobbying (M6 note 29): the engine's arguments on one bill are grouped
// under it (`bill`), with each one's chance and cost shown before it is made.
type Step = { kind: "topics" } | { kind: "subjects"; topic: string; more: boolean } | { kind: "bill"; bill: number };

/** The bills the engine offers arguments on, in its order, with the Fluent key of each one's name. */
export function lobbyBills(topics: readonly TopicView[]): { bill: number; nameKey: string }[] {
  const out: { bill: number; nameKey: string }[] = [];
  for (const t of topics) {
    if (t.bill === undefined || out.some((b) => b.bill === t.bill)) continue;
    out.push({ bill: t.bill, nameKey: t.name_args?.bill_key ?? "" });
  }
  return out;
}

/** The reply on show: which exchange, which page, how much of it is revealed. */
interface Shown {
  id: number | null;
  page: number;
  typed: number;
}

function Dialogue({ v, x, conv, onLost }: { v: TalkView; x: ExchangeView | null; conv: Conversation; onLost: () => void }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const late = useGame((s) => s.talkLate);
  const openTalk = useGame((s) => s.openTalk);
  const talkAbout = useGame((s) => s.talkAbout);
  const goToTalk = useGame((s) => s.goToTalk);
  const setLate = useGame((s) => s.setTalkLate);
  const visit = useGame((s) => s.visit);
  const openPocket = useGame((s) => s.openPocket);
  const stage = useTalkStage((s) => s.stage);
  const street = conv.mode === "street";

  // Opened from the whip's list on a bill: straight at the arguments on it.
  const [step, setStep] = useState<Step>(() => {
    const bill = useTalkFocus.getState().bill;
    return bill !== null ? { kind: "bill", bill } : { kind: "topics" };
  });
  useEffect(() => {
    useTalkFocus.getState().setBill(null);
  }, []);
  const [q, setQ] = useState("");
  const [active, setActive] = useState<string | null>(null);
  // The player's line (their bubble), and the line that has held the stage long enough.
  const asked = useGame((s) => s.talkAsked);
  const setAsked = useGame((s) => s.setTalkAsked);
  const [held, setHeld] = useState<typeof asked>(null);
  // The greeting, as the conversation opened (the engine's view drops it once they have talked today); one
  // found away from them comes when the player reaches them ("Go to them"), before anything is asked.
  const [greeting, setGreeting] = useState(() => v.greeting);
  if (!greeting && v.greeting && !x && !asked) setGreeting(v.greeting);
  const [shown, setShown] = useState<Shown>({ id: x?.id ?? null, page: 0, typed: !x && greeting && !reducedMotion() ? 0 : Infinity });
  const [notedGone, setNotedGone] = useState<number | null>(null);

  const boxRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const veilRef = useRef<HTMLDivElement>(null);
  const layerRef = useRef<HTMLDivElement>(null);
  const personBubble = useRef<HTMLDivElement>(null);
  const playerBubble = useRef<HTMLDivElement>(null);
  /** What to focus after the next render: the first choice, or one by id. */
  const [focusReq, setFocusReq] = useState<{ to: string; n: number }>({ to: "first", n: 1 });
  const focusDone = useRef(0);
  const focus = useCallback((to: string) => setFocusReq((r) => ({ to, n: r.n + 1 })), []);
  const opener = useRef<Element | null>(null);

  // A new reply starts at its first page (the derived-state pattern: no effect, no flash of the old one).
  const xid = x?.id ?? null;
  if (xid !== shown.id) setShown({ id: xid, page: 0, typed: reducedMotion() ? Infinity : 0 });
  // Before any reply: the greeting. Then the latest reply, page by page.
  const pages: Page[] = useMemo(() => (x ? replyPages(tr, x) : greeting ? [greetingPage(tr, greeting)] : []), [x, tr, greeting]);
  // The reply is in (not the one the question was asked after).
  const fresh = asked !== null && x !== null && asked.after !== xid;
  // The player's line is up from the moment they ask until it has held the stage a moment with the reply in.
  const showQuestion = asked !== null && !asked.qShown;
  // The reply waits while the engine answers, and while the player's line is up.
  const waiting = asked !== null && (!fresh || showQuestion);
  const page: Page | null = !waiting && pages.length > 0 ? (pages[Math.min(shown.page, pages.length - 1)] ?? null) : null;
  const full = page ? pageText(page).length : 0;
  const typing = page !== null && shown.typed < full;
  const more = page !== null && shown.page < pages.length - 1;
  const atEnd = x !== null && !waiting && !more;

  // The typewriter (off with reduced motion: the page is all there at once).
  useEffect(() => {
    if (!typing) return;
    const id = window.setInterval(() => setShown((s) => (s.typed >= full ? s : { ...s, typed: Math.min(full, s.typed + TYPE_STEP) })), TYPE_MS);
    return () => window.clearInterval(id);
  }, [typing, full]);

  // The player's line holds the stage a moment from when they ask; once it has and the reply is in, it gives way.
  useEffect(() => {
    if (!asked || asked.qShown) return;
    const id = window.setTimeout(() => setHeld(asked), ASK_HOLD_MS);
    return () => window.clearTimeout(id);
  }, [asked]);
  const lineDone = fresh && !asked.qShown && held === asked;
  useEffect(() => {
    if (lineDone) setAsked((a) => (a ? { ...a, qShown: true } : a));
  }, [lineDone, setAsked]);

  /** Tap, Enter or Space on a reply: reveal the page at once, else the next page. */
  const advance = useCallback(() => {
    if (!page) return;
    if (shown.typed < full) setShown((s) => ({ ...s, typed: Infinity }));
    else if (shown.page < pages.length - 1) {
      focus(shown.page + 1 < pages.length - 1 ? "continue" : "first");
      setShown((s) => ({ ...s, page: s.page + 1, typed: reducedMotion() ? Infinity : 0 }));
    }
  }, [page, shown, full, pages.length, focus]);

  const leave = useCallback(() => void openTalk(null), [openTalk]);

  const ask = (t: TopicView, about: AboutChoiceView | null) => {
    if (busy) return;
    setAsked({ text: questionText(tr, t, about), after: xid, qShown: false });
    // An argument on a bill keeps the bill's arguments in view for the next.
    if (t.bill === undefined) setStep({ kind: "topics" });
    focus("continue");
    // Nothing came of it (the engine refused): the question goes, rather than wait for a reply.
    void talkAbout(t.id, about ? about.about : null).then(() => {
      if ((useGame.getState().exchange?.id ?? null) === xid) setAsked(null);
    });
  };

  const openSubjects = (t: TopicView) => {
    setQ("");
    setStep({ kind: "subjects", topic: t.id, more: false });
    focus("first");
  };
  const openMore = (t: TopicView) => {
    setQ("");
    setStep({ kind: "subjects", topic: t.id, more: true });
    focus("first");
  };
  const back = useCallback(() => {
    if (step.kind === "bill") {
      focus(`bill-${step.bill}`);
      setStep({ kind: "topics" });
      return;
    }
    if (step.kind !== "subjects") return;
    setQ("");
    if (step.more) {
      focus("more");
      setStep({ ...step, more: false });
      return;
    }
    focus(`topic-${step.topic}`);
    setStep({ kind: "topics" });
  }, [step, focus]);

  // ------------------------------------------------------------------ the keyboard

  // Escape steps back from the subjects, else leaves; Enter or Space off any control turns the page.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || document.querySelector('[aria-modal="true"]')) return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        if (step.kind !== "topics") back();
        else leave();
        return;
      }
      const t = e.target;
      const control = t instanceof HTMLElement && t.closest("button, input, select, textarea, a");
      if ((e.key === "Enter" || e.key === " ") && !control && page) {
        e.preventDefault();
        advance();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [step, back, leave, advance, page]);

  /** Arrow keys move between the choices (Home, End: the first, the last). */
  const onListKey = (e: ReactKeyboardEvent) => {
    const list = listRef.current;
    if (!list) return;
    const items = [...list.querySelectorAll<HTMLElement>("[data-choice]")];
    if (items.length === 0) return;
    const i = items.indexOf(document.activeElement as HTMLElement);
    const n = items.length;
    const to = e.key === "ArrowDown" ? (i + 1) % n : e.key === "ArrowUp" ? (i <= 0 ? n - 1 : i - 1) : e.key === "Home" ? 0 : e.key === "End" ? n - 1 : -1;
    if (to < 0 || (e.target instanceof HTMLInputElement && e.key !== "ArrowDown")) return;
    e.preventDefault();
    items[to]?.focus();
  };

  // The keyboard comes into the box when it opens; it goes back to whatever opened it (still there), else the street.
  useLayoutEffect(() => {
    opener.current = document.activeElement;
    return () => {
      const o = opener.current;
      const st = useTalkStage.getState().stage;
      if (!street && o instanceof HTMLElement && o.isConnected && o.getClientRects().length > 0 && o !== document.body) o.focus({ preventScroll: true });
      else st?.focus();
    };
    // Once per conversation.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  // After a render that asked for it, or that took away the focused choice: focus a choice.
  useEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const want = focusReq.n !== focusDone.current ? focusReq.to : null;
    const lost = document.activeElement === document.body || document.activeElement === null;
    if (!want && !lost) return;
    const items = [...list.querySelectorAll<HTMLElement>("[data-choice], input")];
    const pick = (want && want !== "first" ? items.find((el) => el.dataset.choice === want) : null) ?? items.find((el) => el.getAttribute("aria-disabled") !== "true") ?? items[0];
    if (!pick) return;
    if (want === "continue" && !items.some((el) => el.dataset.choice === "continue")) {
      // The reply is not in yet: keep the request for when it is.
      if (busy || waiting) return;
    }
    focusDone.current = focusReq.n;
    pick.focus({ preventScroll: true });
  });

  // ------------------------------------------------------------------ the street

  // Frame the two speakers (dims the rest, pauses walking) for as long as the conversation is in the street.
  useLayoutEffect(() => {
    if (!street) return;
    if (!stage) {
      onLost();
      return;
    }
    const box = boxRef.current;
    stage.begin(conv.key, box ? box.getBoundingClientRect().top : window.innerHeight);
    return () => stage.end();
    // The person keeps their walker when met (a new key): framing goes on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [street, stage]);

  const notedOn = atEnd && x !== null && x.learned.length > 0 && notedGone !== x.id;
  // Bubbles over the sprites, the light on the speakers: on every drawn frame (the camera easing) and every render.
  const place = useCallback(() => {
    if (!street || !stage) return;
    const a = stage.anchors();
    if (!a) {
      onLost();
      return;
    }
    const box = boxRef.current?.getBoundingClientRect();
    const veil = veilRef.current;
    if (veil) {
      veil.style.setProperty("--dlg-x", `${Math.round((a.player.x + a.person.x) / 2)}px`);
      veil.style.setProperty("--dlg-y", `${Math.round((a.player.y + a.person.y) / 2 + a.scale * 0.8)}px`);
      veil.style.setProperty("--dlg-r", `${Math.round(Math.max(64, a.scale * 1.6))}px`);
    }
    const bounds = { left: 8, top: Math.max(8, a.bounds.top + 8) + (notedOn ? 52 : 0), right: window.innerWidth - 8, bottom: (box ? box.top : a.bounds.bottom) - 8 };
    if (personBubble.current) placeBubble(personBubble.current, a.person, a.scale * 1.5, bounds);
    if (playerBubble.current) placeBubble(playerBubble.current, a.player, a.scale * 1.5, bounds);
  }, [street, stage, onLost, notedOn]);
  useLayoutEffect(() => {
    place();
  });
  useEffect(() => {
    if (!street || !stage) return;
    const off = stage.onFrame(place);
    window.addEventListener("resize", place);
    return () => {
      off();
      window.removeEventListener("resize", place);
    };
  }, [street, stage, place]);
  // The box's height: the street frames the speakers above it; a portrait stands on it.
  useEffect(() => {
    const box = boxRef.current;
    if (!box || typeof ResizeObserver !== "function") return;
    const ro = new ResizeObserver(() => {
      const r = box.getBoundingClientRect();
      layerRef.current?.style.setProperty("--dlg-box-h", `${Math.round(window.innerHeight - r.top)}px`);
      if (!street || !stage) return;
      stage.cover(r.top);
      place();
    });
    ro.observe(box);
    return () => ro.disconnect();
  }, [street, stage, place]);
  // A new step, or the end of a reply (the outcome above the choices): the box's list from the top.
  useLayoutEffect(() => {
    if (listRef.current) listRef.current.scrollTop = 0;
  }, [step, atEnd]);
  // The person speaks while their words come out.
  useEffect(() => {
    if (!street || !stage) return;
    stage.speaking(typing ? "person" : null);
  }, [street, stage, typing]);

  // ------------------------------------------------------------------ the notebook toast

  useEffect(() => {
    if (!notedOn || !x) return;
    const id = window.setTimeout(() => setNotedGone(x.id), NOTED_MS);
    return () => window.clearTimeout(id);
  }, [notedOn, x]);

  // ------------------------------------------------------------------ what is shown

  const name = displayName(tr, v.person);
  const label = v.person.name ? tr.t("ui-talk-with", { name: v.person.name }) : tr.t("ui-talk-with-stranger");
  const role = v.person.name ? roleText(tr, v.person) : tr.t("ui-dlg-stranger");
  const cost = costText(tr, v);
  // The engine is answering: the person is thinking while the player's line is up.
  const thinking = asked !== null && !fresh;
  const replyLive = page ? tr.t("ui-dlg-says", { name, line: pageText(page) }) : "";
  // A reason every topic shares (out of time, not here) is said once, above the choices.
  const shared = new Set<string>([...(v.out_of_time ? ["talk-why-time", "talk-why-late-used"] : []), ...(!v.here ? ["talk-why-away"] : [])]);
  const statusId = "dlg-status";

  const bubbles = (
    <>
      {asked && showQuestion && <Bubble who="player" page={asked.text} side={!street} testId="talk-you" bubbleRef={playerBubble} />}
      {(page || thinking) && (
        <Bubble who="person" page={page} typed={shown.typed} more={more && !typing} thinking={!page} side={!street} onAdvance={advance} testId="talk-bubble" bubbleRef={personBubble} />
      )}
    </>
  );

  const choice = (id: string, opts: { label: string; onChoose: () => void; disabled?: boolean; why?: string | null; odds?: number | null; opens?: boolean; asked?: boolean; kind?: string; testId?: string; describedBy?: string | undefined; costs?: string | null }) => {
    const disabled = !!opts.disabled || busy;
    const whyId = opts.why ? `dlg-why-${id.replace(/\W/g, "-")}` : undefined;
    const costId = opts.costs ? `dlg-cost-${id.replace(/\W/g, "-")}` : undefined;
    return (
      <li key={id} className={`dlg-item${opts.kind ? ` dlg-item-${opts.kind}` : ""}`} data-testid={opts.testId}>
        <button
          type="button"
          className="dlg-choice"
          data-choice={id}
          tabIndex={(active ?? "") === id ? 0 : -1}
          aria-disabled={disabled || undefined}
          aria-describedby={[costId, whyId ?? opts.describedBy].filter(Boolean).join(" ") || undefined}
          onFocus={() => setActive(id)}
          onClick={() => {
            if (!disabled) opts.onChoose();
          }}
        >
          <span className="dlg-choice-label">{opts.label}</span>
          {opts.asked && <span className="dlg-tag dlg-tag-quiet">{tr.t("ui-talk-asked")}</span>}
          {opts.odds !== null && opts.odds !== undefined && (
            <span className="dlg-tag dlg-odds" data-testid="talk-odds" title={tr.t("ui-talk-odds", { pct: Math.round(opts.odds * 100) })}>
              <span aria-hidden="true">{tr.t("ui-dlg-odds", { pct: Math.round(opts.odds * 100) })}</span>
              <span className="visually-hidden">{tr.t("ui-talk-odds", { pct: Math.round(opts.odds * 100) })}</span>
            </span>
          )}
          {opts.opens && (
            <span className="dlg-opens" aria-hidden="true">
              ›
            </span>
          )}
        </button>
        {opts.costs && (
          <p className="dlg-costs" id={costId} data-testid="talk-costs">
            {opts.costs}
          </p>
        )}
        {opts.why && (
          <p className="dlg-why" id={whyId} data-testid="talk-why">
            {tr.t(opts.why)}
          </p>
        )}
      </li>
    );
  };

  const leadGo = (l: LeadView, i: number) => {
    const dest = l.route?.to ?? null;
    if (!dest || l.route?.here) return null;
    return choice(`lead-${i}`, {
      label: tr.t("ui-talk-go", { place: placeName(tr, dest) }),
      kind: "go",
      testId: "talk-lead-go",
      onChoose: () => {
        void openTalk(null);
        // The lead's place: go there and open it (free).
        void visit(dest.location);
      },
    });
  };

  const bills = lobbyBills(v.topics);
  const topicItems = () => [
    ...bills.map(({ bill, nameKey }) => {
      const args = v.topics.filter((t) => t.bill === bill);
      const available = args.some((t) => t.available);
      return choice(`bill-${bill}`, {
        label: tr.t("ui-dlg-lobby-bill", { bill: tr.t(nameKey) }),
        disabled: !available,
        why: available ? null : (args[0]?.unavailable_key ?? null),
        opens: true,
        testId: `talk-bill-${bill}`,
        onChoose: () => {
          setStep({ kind: "bill", bill });
          focus("first");
        },
      });
    }),
    ...v.topics.filter((t) => t.bill === undefined).map((t) => {
      const r = topicRow(t);
      const why = r.why && !shared.has(r.why) ? r.why : null;
      return choice(`topic-${t.id}`, {
        label: topicLabel(tr, t),
        disabled: !r.available,
        why,
        describedBy: !r.available && !why ? statusId : undefined,
        odds: r.odds,
        opens: r.opens,
        asked: t.asked_this_week,
        testId: `talk-topic-${t.id}`,
        onChoose: () => (r.opens ? openSubjects(t) : ask(t, null)),
      });
    }),
  ];

  /** The arguments on one bill (lobbying): each with its chance and what it costs, before it is made. */
  const billStep = (bill: number) => {
    const args = v.topics.filter((t) => t.bill === bill);
    const nameKey = lobbyBills(v.topics).find((b) => b.bill === bill)?.nameKey ?? "";
    return (
      <>
        <div className="dlg-step-head">
          <button type="button" className="dlg-back" onClick={back} data-testid="talk-back">
            <span aria-hidden="true">‹ </span>
            {tr.t("ui-dlg-back")}
          </button>
          <h3 className="dlg-step-title" id="dlg-step-title">
            {tr.t("ui-dlg-lobby-bill", { bill: tr.t(nameKey) })}
          </h3>
        </div>
        <p className="dlg-hint">{tr.t("ui-dlg-lobby-hint")}</p>
        <ul className="dlg-choices" aria-labelledby="dlg-step-title" data-testid="talk-arguments">
          {page &&
            more &&
            choice("continue", {
              label: tr.t("ui-dlg-continue", { n: shown.page + 1, total: pages.length }),
              kind: "continue",
              testId: "talk-continue",
              onChoose: advance,
            })}
          {args.map((t) =>
            choice(`topic-${t.id}`, {
              label: topicLabel(tr, t),
              disabled: !t.available,
              why: t.available || shared.has(t.unavailable_key ?? "") ? null : t.unavailable_key,
              describedBy: !t.available ? statusId : undefined,
              odds: t.odds,
              costs: t.costs && t.costs.length > 0 ? t.costs.map((c) => tr.event(c.key, c.args)).join(" · ") : null,
              testId: "talk-argument",
              onChoose: () => ask(t, null),
            }),
          )}
        </ul>
      </>
    );
  };
  // On a bill with no argument left (talked out this week, or settled): the topics, and a word why.
  const onBill = step.kind === "bill" && v.topics.some((t) => t.bill === step.bill) ? step.bill : null;
  const billDone = step.kind === "bill" && onBill === null;

  const topic = step.kind === "subjects" ? (v.topics.find((t) => t.id === step.topic) ?? null) : null;
  const subjects = () => {
    if (!topic) return null;
    const more = step.kind === "subjects" && step.more;
    const { likely, others } = subjectSplit(topic.about);
    // With no likely subject, the rest are the list (nothing to go behind).
    const listed = more || likely.length === 0 ? others : likely;
    const groups = subjectGroups(tr, listed, q);
    const searching = more || listed.length >= SEARCH_FROM;
    const offerMore = !more && likely.length > 0 && others.length > 0;
    return (
      <>
        <div className="dlg-step-head">
          <button type="button" className="dlg-back" onClick={back} data-testid="talk-back">
            <span aria-hidden="true">‹ </span>
            {tr.t("ui-dlg-back")}
          </button>
          <h3 className="dlg-step-title" id="dlg-step-title">
            {tr.t(topic.name_key)}
          </h3>
        </div>
        {more && <p className="dlg-hint">{tr.t("ui-dlg-something-else-hint")}</p>}
        {searching && (
          <input
            type="search"
            className="input dlg-search"
            value={q}
            placeholder={tr.t("ui-dlg-search")}
            aria-label={tr.t("ui-dlg-search")}
            onChange={(e) => setQ(e.target.value)}
            data-testid="talk-subject-search"
          />
        )}
        <ul className="dlg-choices" aria-labelledby="dlg-step-title" data-testid="talk-subjects">
          {!topic.about_required &&
            !q &&
            !more &&
            choice("none", {
              label: tr.t("ui-talk-about-none"),
              disabled: !topic.available,
              why: topic.available || shared.has(topic.unavailable_key ?? "") ? null : topic.unavailable_key,
              odds: topic.odds,
              testId: "talk-subject",
              onChoose: () => ask(topic, null),
            })}
          {groups.map(([kind, items]) => {
            const rows = items.map((c) =>
              choice(`about-${JSON.stringify(c.about)}`, {
                label: aboutText(tr, c.label, c.label_key, c.label_args),
                disabled: !c.available,
                why: c.available || shared.has(c.unavailable_key ?? "") ? null : c.unavailable_key,
                describedBy: !c.available ? statusId : undefined,
                odds: c.odds,
                testId: "talk-subject",
                onChoose: () => ask(topic, c),
              }),
            );
            return groups.length > 1 ? (
              <li key={kind} className="dlg-group">
                <p className="dlg-group-name" id={`dlg-kind-${kind}`}>
                  {tr.t(`ui-talk-kind-${kind}`)}
                </p>
                <ul className="dlg-choices" aria-labelledby={`dlg-kind-${kind}`}>
                  {rows}
                </ul>
              </li>
            ) : (
              <Fragment key={kind}>{rows}</Fragment>
            );
          })}
          {groups.length === 0 && q && <li className="dlg-none">{tr.t("ui-dlg-search-none", { q })}</li>}
          {offerMore &&
            !q &&
            choice("more", {
              label: tr.t("ui-dlg-something-else"),
              opens: true,
              testId: "talk-something-else",
              onChoose: () => openMore(topic),
            })}
        </ul>
      </>
    );
  };

  const away = !v.here;
  const dest = v.route?.to ?? null;

  return (
    <div ref={layerRef} className={`dlg-layer dlg-${conv.mode}`} data-mode={conv.mode} data-testid="talk-layer">
      {street ? (
        // The street, dimmed but for the two speakers; a tap turns the page.
        <div ref={veilRef} className="dlg-veil" data-testid="talk-veil" onClick={advance} aria-hidden="true" />
      ) : (
        <div className="dlg-scene" onClick={advance} aria-hidden="true">
          <div className="dlg-portrait-row">
            <Portrait variant={conv.look} className="dlg-face" />
            <div className="dlg-portrait-talk">{bubbles}</div>
          </div>
        </div>
      )}
      {street && bubbles}

      {notedOn && x && (
        <div className="dlg-noted" role="status" data-testid="talk-noted">
          <button
            type="button"
            className="dlg-noted-open"
            onClick={() => {
              void openTalk(null);
              openPocket("notebook");
            }}
            data-testid="talk-noted-open"
          >
            <span aria-hidden="true">✎ </span>
            {x.learned.length > 1 ? tr.t("ui-dlg-noted-more", { what: learnedText(tr, x.learned[0] as ExchangeView["learned"][number]), more: x.learned.length - 1 }) : tr.t("ui-dlg-noted", { what: learnedText(tr, x.learned[0] as ExchangeView["learned"][number]) })}
            <span className="visually-hidden"> {tr.t("ui-dlg-noted-open")}</span>
          </button>
          <button type="button" className="dlg-noted-x" aria-label={tr.t("ui-dismiss")} onClick={() => setNotedGone(x.id)}>
            ×
          </button>
        </div>
      )}

      <div className="dlg-box-wrap" ref={boxRef}>
        <div className="dlg-plate dlg-frame">
          <h2 className="dlg-name" id="dlg-name">
            {name}
          </h2>
        </div>
        <section className="dlg-box dlg-frame" role="dialog" aria-label={label} aria-busy={busy || undefined} data-testid="talk-panel" data-leads={x?.leads.length ?? 0}>
          <header className="dlg-head">
            <p className="dlg-sub">
              {role}
              {role && " · "}
              <span data-testid="talk-opinion">{tr.t(`ui-talk-opinion-${v.opinion_band}`)}</span>
            </p>
            {v.here && cost && (
              <p className="dlg-cost" data-testid="talk-cost">
                <span aria-hidden="true">{cost.short}</span>
                <span className="visually-hidden">{cost.long}</span>
              </p>
            )}
            {v.out_of_time && <p className="dlg-cost dlg-cost-out">{tr.t("ui-dlg-out-of-time")}</p>}
            <button type="button" className="dlg-close" onClick={leave} aria-label={tr.t("ui-close")}>
              ✕
            </button>
          </header>
          {/* The reply as said, for screen readers (the bubble is drawn for the eyes). */}
          <p className="visually-hidden" aria-live="polite" data-testid="talk-reply-live">
            {replyLive}
          </p>
          <div className="dlg-body" ref={listRef} onKeyDown={onListKey}>
            {away && (
              <p className="dlg-status" id={statusId} data-testid="talk-away">
                {dest ? tr.t("ui-talk-away", { place: placeName(tr, dest) }) : tr.t("ui-talk-nowhere")}
              </p>
            )}
            {v.here && v.out_of_time && (
              <p className="dlg-status dlg-status-out" id={statusId} role="status" data-testid="talk-out-of-time">
                <span aria-hidden="true">⌛ </span>
                {v.late_night ? tr.t("talk-why-late-used") : tr.t("ui-talk-out-of-time")}
              </p>
            )}
            {atEnd && x && (
              <p className="dlg-status dlg-outcome" data-testid="talk-outcome">
                {tr.event(x.outcome_key, x.outcome_args)}
              </p>
            )}
            {billDone && (
              <p className="dlg-status" data-testid="talk-bill-done">
                {tr.t("ui-dlg-lobby-done")}
              </p>
            )}
            {onBill !== null ? (
              billStep(onBill)
            ) : step.kind === "subjects" && topic ? (
              subjects()
            ) : (
              <ul className="dlg-choices dlg-topics" aria-label={tr.t("ui-dlg-choices")} data-testid="talk-topics">
                {page &&
                  more &&
                  choice("continue", {
                    label: tr.t("ui-dlg-continue", { n: shown.page + 1, total: pages.length }),
                    kind: "continue",
                    testId: "talk-continue",
                    onChoose: advance,
                  })}
                {away &&
                  dest &&
                  choice("go", {
                    label: tr.t("ui-talk-go", { place: placeName(tr, dest) }),
                    kind: "go",
                    testId: "talk-go",
                    onChoose: () => void goToTalk(),
                  })}
                {atEnd && x?.leads.map(leadGo)}
                {topicItems()}
                {choice("goodbye", { label: tr.t("ui-dlg-goodbye"), kind: "bye", testId: "talk-goodbye", onChoose: leave })}
              </ul>
            )}
            {v.late_night_possible && (
              <label className="check dlg-late">
                <input type="checkbox" checked={late} disabled={busy} onChange={(e) => void setLate(e.target.checked)} data-testid="talk-late" />
                {tr.t("ui-talk-late")}
              </label>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
