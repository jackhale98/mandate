// The content workbench's designer page (owner, 2026-11-19), dev builds
// only (`?dev=1` under `npm run dev`, or a `vite build --mode devtools`
// bundle). It lists every dilemma, situational policy, storyline, scene,
// overheard line, conversation topic, line and voice, says whether each
// could fire right now and why (the engine's evaluators, as a tree with the
// values they read), and previews its text with its real arguments.
//
// The page runs its own engine: a new game from a seed, or a copy of the
// current run loaded from its save (never written back). Passing time here
// changes only this sandbox. A developer tool, so its labels are plain
// English rather than locale keys; the content it previews is rendered
// through Fluent like the game's.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { engine } from "../engine/client";
import { Translator } from "../i18n/fluent";
import { saveStore } from "../persistence/indexedDbStore";
import {
  DEV_KINDS,
  SPEAKER_KINDS,
  devCatalog,
  devExplain,
  endTurns,
  englishFtl,
  isDevless,
  newGame,
  type DevKind,
} from "./devClient";
import { ExplainTree } from "./ExplainTree";
import type { DevCatalogView } from "./protocol/DevCatalogView";
import type { DevExplainView } from "./protocol/DevExplainView";
import type { DevItemView } from "./protocol/DevItemView";
import type { DevTextView } from "./protocol/DevTextView";
import "./designer.css";

type Status = { kind: "starting" } | { kind: "devless" } | { kind: "ready" } | { kind: "error"; message: string };
type Verdict = "all" | "yes" | "no";

function render(tr: Translator | null, key: string | null | undefined, args: Record<string, string> = {}): string {
  if (!key) return "";
  if (!tr) return key;
  return tr.event(key, args);
}

function seedFromUrl(): string {
  const s = new URLSearchParams(location.search).get("seed");
  return s && /^\d+$/.test(s) ? s : "2026";
}

function TextPreview({ t, tr }: { t: DevTextView; tr: Translator | null }) {
  const args = Object.entries(t.args);
  const text = render(tr, t.key, t.args);
  return (
    <div className="dev-text">
      <div className="dev-text-slot">
        {t.slot} <code>{t.key}</code> <span className="dev-count">{text.length} chars</span>
      </div>
      <p className="dev-text-body">{text}</p>
      {args.length > 0 && (
        <details>
          <summary>arguments ({args.length})</summary>
          <table className="dev-args">
            <tbody>
              {args.map(([k, v]) => (
                <tr key={k}>
                  <th>${k}</th>
                  <td>{v}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </details>
      )}
    </div>
  );
}

function Preview({ x, tr }: { x: DevExplainView; tr: Translator | null }) {
  const d = x.decision;
  return (
    <section className="dev-card" aria-label="preview">
      <h3>
        Preview {x.forced && <span className="dev-badge dev-badge-warn">forced cast: conditions not met</span>}
      </h3>
      {x.cast.length > 0 && (
        <p className="dev-cast">
          {x.cast.map((c) => (
            <span key={c.role} className="dev-value">
              {c.role} = <b>{c.actor}</b>
            </span>
          ))}
        </p>
      )}
      {x.texts.map((t, i) => (
        <TextPreview key={`${t.slot}-${i}`} t={t} tr={tr} />
      ))}
      {d && (
        <div className="dev-options">
          <h4>Options</h4>
          {d.options.map((o) => (
            <div key={o.id} className="dev-option">
              <div className="dev-option-label">
                <b>{render(tr, o.label_key, d.args)}</b> <code>{o.id}</code>
                {o.default && <span className="dev-badge">default</span>}
                <span className="dev-count">{render(tr, o.label_key, d.args).length} chars</span>
              </div>
              <ul className="dev-lines">
                {o.costs.map((l, i) => (
                  <li key={`c${i}`} data-tone="cost">
                    − {render(tr, l.key, l.args)}
                  </li>
                ))}
                {o.gains.map((l, i) => (
                  <li key={`g${i}`} data-tone="gain">
                    + {render(tr, l.key, l.args)}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function ItemRow({
  item,
  tr,
  selected,
  onPick,
}: {
  item: DevItemView;
  tr: Translator | null;
  selected: boolean;
  onPick: () => void;
}) {
  const title = render(tr, item.title_key);
  return (
    <li>
      <button type="button" className="dev-item" aria-pressed={selected} data-ok={item.could_fire} onClick={onPick}>
        <span className={item.could_fire ? "dev-mark dev-mark-ok" : "dev-mark dev-mark-fail"} aria-hidden>
          {item.could_fire ? "✓" : "✗"}
        </span>
        <span className="dev-item-main">
          <span className="dev-item-id">{item.id}</span>
          <span className="dev-item-meta">
            {item.kind} · {item.group}
            {!item.could_fire && item.blocked_by && <> · blocked by {item.blocked_by}</>}
          </span>
          {title && <span className="dev-item-title">{title}</span>}
        </span>
      </button>
    </li>
  );
}

export function DesignerPage() {
  const client = useMemo(() => engine(), []);
  const [status, setStatus] = useState<Status>({ kind: "starting" });
  const [tr, setTr] = useState<Translator | null>(null);
  const [catalog, setCatalog] = useState<DevCatalogView | null>(null);
  const [seed, setSeed] = useState(seedFromUrl);
  const [source, setSource] = useState("");
  const [kind, setKind] = useState<DevKind | "all">("all");
  const [verdict, setVerdict] = useState<Verdict>("all");
  const [query, setQuery] = useState("");
  const [picked, setPicked] = useState<{ kind: string; id: string } | null>(null);
  const [speaker, setSpeaker] = useState<number | null>(null);
  const [place, setPlace] = useState<number | null>(null);
  const [x, setX] = useState<DevExplainView | null>(null);
  const [forced, setForced] = useState(false);
  const [busy, setBusy] = useState(false);
  const [hasRun, setHasRun] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setCatalog(await devCatalog(client));
      setStatus({ kind: "ready" });
    } catch (e) {
      setStatus(isDevless(e) ? { kind: "devless" } : { kind: "error", message: String(e) });
    }
  }, [client]);

  const start = useCallback(
    async (s: string) => {
      setBusy(true);
      try {
        await newGame(client, s);
        setSource(`new game, seed ${s}`);
        await refresh();
      } catch (e) {
        setStatus({ kind: "error", message: String(e) });
      } finally {
        setBusy(false);
      }
    },
    [client, refresh],
  );

  // Once per page (StrictMode runs effects twice in development).
  const booted = useRef(false);
  useEffect(() => {
    if (booted.current) return;
    booted.current = true;
    void (async () => {
      try {
        await client.ready;
        setTr(new Translator("en", await englishFtl(client)));
        const runs = await saveStore()
          .listRuns()
          .catch(() => []);
        setHasRun(runs.some((r) => r.status === "active"));
        await start(seedFromUrl());
      } catch (e) {
        setStatus({ kind: "error", message: String(e) });
      }
    })();
  }, [client, start]);

  const loadCurrentRun = async () => {
    setBusy(true);
    try {
      const store = saveStore();
      const run = (await store.listRuns()).find((r) => r.status === "active");
      const rec = run && (await store.getRun(run.runId));
      if (!rec) throw new Error("no saved run on this device");
      // A copy: the workbench's engine never saves.
      await client.importSave(rec.bytes.slice());
      setSource(`current run ${run.name ?? run.runId.slice(0, 8)} (a copy)`);
      await refresh();
    } catch (e) {
      setStatus({ kind: "error", message: String(e) });
    } finally {
      setBusy(false);
    }
  };

  const pass = async (n: number) => {
    setBusy(true);
    try {
      await endTurns(client, n);
      await refresh();
    } catch (e) {
      setStatus({ kind: "error", message: String(e) });
    } finally {
      setBusy(false);
    }
  };

  // Explain the picked item whenever it, the speaker, the place or the game changes.
  useEffect(() => {
    if (!picked || status.kind !== "ready") return;
    let live = true;
    devExplain(client, picked.kind, picked.id, speaker, place)
      .then((v) => {
        if (live) setX(v);
      })
      .catch((e: unknown) => {
        if (live) setStatus({ kind: "error", message: String(e) });
      });
    return () => {
      live = false;
    };
  }, [client, picked, speaker, place, catalog, status.kind]);

  const items = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (catalog?.items ?? []).filter((i) => {
      if (kind !== "all" && i.kind !== kind) return false;
      if (verdict === "yes" && !i.could_fire) return false;
      if (verdict === "no" && i.could_fire) return false;
      if (!q) return true;
      const title = render(tr, i.title_key).toLowerCase();
      return i.id.toLowerCase().includes(q) || i.group.toLowerCase().includes(q) || title.includes(q);
    });
  }, [catalog, kind, verdict, query, tr]);

  const counts = useMemo(() => {
    const all = catalog?.items ?? [];
    return { all: all.length, firing: all.filter((i) => i.could_fire).length };
  }, [catalog]);

  const pick = (i: DevItemView) => {
    setPicked({ kind: i.kind, id: i.id });
    setX(null);
    setForced(false);
  };

  return (
    <div className="dev-page">
      <header className="dev-header">
        <div className="dev-title">
          <h1>Content workbench</h1>
          <span className="dev-badge">dev build</span>
          {catalog && (
            <span className="dev-where">
              {catalog.town} · {catalog.player} · {catalog.date.year}-{String(catalog.date.month).padStart(2, "0")}-
              {String(catalog.date.day).padStart(2, "0")} · {source} · content {catalog.content_hash.slice(0, 8)}
            </span>
          )}
        </div>
        <form
          className="dev-controls"
          onSubmit={(e) => {
            e.preventDefault();
            void start(seed);
          }}
        >
          <label>
            Seed <input value={seed} inputMode="numeric" onChange={(e) => setSeed(e.target.value.replace(/\D/g, ""))} />
          </label>
          <button type="submit" disabled={busy || !seed}>
            New game
          </button>
          <button type="button" disabled={busy || !hasRun} onClick={() => void loadCurrentRun()}>
            Load current run
          </button>
          <span className="dev-sep" />
          <span>Pass time:</span>
          {[1, 4, 13].map((n) => (
            <button key={n} type="button" disabled={busy || status.kind !== "ready"} onClick={() => void pass(n)}>
              {n} {n === 1 ? "turn" : "turns"}
            </button>
          ))}
        </form>
      </header>

      {status.kind === "starting" && <p className="dev-note">Starting the engine…</p>}
      {status.kind === "devless" && (
        <div className="dev-card dev-devless" role="status">
          <h2>This engine was built without the dev tools</h2>
          <p>
            The workbench asks the engine to explain content, and only an engine built with the <code>dev-tools</code>{" "}
            feature answers. Build one (the release build never has it):
          </p>
          <pre>
            cargo build -p mandate-wasm --target wasm32-unknown-unknown --profile release-wasm --features dev-tools{"\n"}
            wasm-bindgen target/wasm32-unknown-unknown/release-wasm/mandate_wasm.wasm --out-dir web/src/engine/pkg
            --target web --omit-default-module-path
          </pre>
        </div>
      )}
      {status.kind === "error" && (
        <p className="dev-card dev-error" role="alert">
          {status.message}
        </p>
      )}

      {status.kind === "ready" && catalog && (
        <main className="dev-main">
          <aside className="dev-list" aria-label="content items">
            <div className="dev-filters">
              <input
                type="search"
                placeholder="Search ids, groups, titles"
                aria-label="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              <select aria-label="kind" value={kind} onChange={(e) => setKind(e.target.value as DevKind | "all")}>
                <option value="all">all kinds</option>
                {DEV_KINDS.map((k) => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </select>
              <select aria-label="verdict" value={verdict} onChange={(e) => setVerdict(e.target.value as Verdict)}>
                <option value="all">any verdict</option>
                <option value="yes">could fire now</option>
                <option value="no">blocked now</option>
              </select>
            </div>
            <p className="dev-note">
              {items.length} shown · {counts.firing} of {counts.all} could fire now
            </p>
            <ul className="dev-items">
              {items.map((i) => (
                <ItemRow
                  key={`${i.kind}:${i.id}`}
                  item={i}
                  tr={tr}
                  selected={picked?.kind === i.kind && picked.id === i.id}
                  onPick={() => pick(i)}
                />
              ))}
            </ul>
          </aside>
          <section className="dev-detail" aria-label="explanation of the item">
            {!picked && <p className="dev-note">Pick an item to see whether it could fire now, and why.</p>}
            {picked && !x && <p className="dev-note">Asking the engine…</p>}
            {x && (
              <>
                <div className="dev-card">
                  <h2>
                    <span className={x.could_fire ? "dev-mark dev-mark-ok" : "dev-mark dev-mark-fail"} aria-hidden>
                      {x.could_fire ? "✓" : "✗"}
                    </span>{" "}
                    {x.id}
                  </h2>
                  <p className="dev-verdict" data-ok={x.could_fire}>
                    {x.kind} · {x.could_fire ? "could fire now" : "cannot fire now"}
                  </p>
                  {SPEAKER_KINDS.includes(x.kind as DevKind) && (
                    <label className="dev-pick">
                      Speaker{" "}
                      <select
                        aria-label="speaker"
                        value={x.speaker ?? ""}
                        onChange={(e) => setSpeaker(e.target.value ? Number(e.target.value) : null)}
                      >
                        {catalog.speakers.map((s) => (
                          <option key={s.id} value={s.id}>
                            {s.name} ({s.role || "out of office"})
                          </option>
                        ))}
                      </select>
                    </label>
                  )}
                  {x.kind === "overheard" && (
                    <label className="dev-pick">
                      Place{" "}
                      <select
                        aria-label="place"
                        value={x.place ?? ""}
                        onChange={(e) => setPlace(e.target.value ? Number(e.target.value) : null)}
                      >
                        {catalog.places.map((p) => (
                          <option key={p.id} value={p.id}>
                            {p.name}
                          </option>
                        ))}
                      </select>
                    </label>
                  )}
                </div>
                {x.could_fire || forced ? (
                  <Preview x={x} tr={tr} />
                ) : (
                  <button type="button" className="dev-force" onClick={() => setForced(true)}>
                    Force preview (renders the text and options without changing anything)
                  </button>
                )}
                <section className="dev-card" aria-label="why">
                  <h3>Why</h3>
                  <ExplainTree tree={x.tree} />
                </section>
              </>
            )}
          </section>
        </main>
      )}
    </div>
  );
}
