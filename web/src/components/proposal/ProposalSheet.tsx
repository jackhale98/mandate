// The proposal sheet (DEC-092, task F6; M6_ENGINE_CONTRACTS §2.12): a
// policy-family proposal as the engine's preview gives it. Its choices, each
// axis an option list whose every option carries the cost, support and
// estimate of the whole package with it swapped in (IMPACT_ESTIMATE §6: the
// estimate's own lines, its ranges as the engine wrote them); the sliders;
// who brings a ready version and what it costs the player; why it is on the
// table; the lead toward knowing more; and the draft or amend command. Below
// "known" it is a stub: the title, the reason, the sponsors' roles and the
// lead (DEC-091). Nothing here computes an outcome: a choice asks the engine
// again, and the command is the engine's to accept.
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import type { AxisPreviewView } from "../../engine/protocol/AxisPreviewView";
import type { BillView } from "../../engine/protocol/BillView";
import type { FamilyPreviewView } from "../../engine/protocol/FamilyPreviewView";
import type { OptionPreviewView } from "../../engine/protocol/OptionPreviewView";
import type { PolicyPreviewView } from "../../engine/protocol/PolicyPreviewView";
import type { PriceView } from "../../engine/protocol/PriceView";
import type { ProposalView } from "../../engine/protocol/ProposalView";
import type { SponsorTermsView } from "../../engine/protocol/SponsorTermsView";
import type { SupportView } from "../../engine/protocol/SupportView";
import type { Translator } from "../../i18n/fluent";
import { formatMoney } from "../../i18n/format";
import { useGame } from "../../state/gameStore";
import { ImpactLines, SupportBars, impactLine } from "../BillCard";
import { capital } from "../DecisionSheet";
import { personName } from "../Knowing";
import { ParamSliders } from "../ParamSliders";
import { TradeOffs } from "../TradeOffs";
import { Lead } from "../notebook/Leads";
import { postTitle } from "../posts";
import { useSheetClose } from "../useSheetClose";
import { useProposal } from "./proposalStore";
import "./proposal.css";

type Tr = Translator;

/** A proposal's title, with its bound names (the engine's arguments). */
export function proposalTitle(tr: Tr, p: Pick<ProposalView, "name_key" | "args">): string {
  return capital(tr.event(p.name_key, p.args));
}

/** A bill's name: a family bill's key takes its proposal's arguments. */
export function billName(tr: Tr, b: Pick<BillView, "name_key" | "family">): string {
  return b.family ? capital(tr.event(b.name_key, b.family.args)) : tr.t(b.name_key);
}

/** Support in a word, with the engine's figure where the player knows it. */
export function supportText(tr: Tr, s: SupportView | null | undefined): string {
  if (!s) return "";
  const band = tr.t(`support-band-${s.band}`);
  if (s.overall === null) return band;
  const n = Math.round(s.overall * 100);
  return tr.t("ui-proposal-support-figure", { band, figure: n > 0 ? `+${n}` : String(n) });
}

/** A sponsor's price in words: `_key` arguments are keys, and a count picks its plural. */
export function priceText(tr: Tr, p: PriceView): string {
  const args: Record<string, string | number> = {};
  for (const [k, v] of Object.entries(p.args)) {
    if (k.endsWith("_key")) args[k.slice(0, -4)] = tr.t(v);
    else args[k] = v;
  }
  if (p.kind === "favour") args.count = p.favours;
  return tr.t(p.key, args);
}

function costLine(tr: Tr, cost: number, share: number, months: number): string {
  const parts = [tr.t("ui-proposal-cost", { amount: formatMoney(cost) })];
  if (share > 0) parts.push(tr.t("ui-proposal-share", { pct: (share * 100).toFixed(1) }));
  parts.push(tr.t("ui-impl-months", { n: months }));
  return parts.join(" · ");
}

/** The names of a version's options, axis by axis, as the preview names them. */
function termsText(tr: Tr, fam: FamilyPreviewView, s: SponsorTermsView): string {
  const opts = s.preset.map((c) => {
    const axis = fam.axes.find((a) => a.id === c.axis);
    const o = axis?.options.find((x) => x.id === c.option);
    return o ? tr.t(o.name_key) : "";
  });
  const sliders = s.params.map((v) => {
    const d = fam.param_defs.find((x) => x.id === v.id);
    return d ? `${tr.t(d.name_key)} ${v.value} ${tr.t(d.unit_key)}` : "";
  });
  return [...opts, ...sliders].filter(Boolean).join(" · ");
}

function whoText(tr: Tr, s: Pick<SponsorTermsView, "person">): string {
  const role = s.person.role_key ? tr.t(s.person.role_key) : "";
  return s.person.name ? [s.person.name, role].filter(Boolean).join(", ") : role || personName(tr, "");
}

// --- The parts of the sheet -----------------------------------------------------------

function Option({ o, axis, name, busy, onPick }: { o: OptionPreviewView; axis: AxisPreviewView; name: string; busy: boolean; onPick: () => void }) {
  const tr = useGame((s) => s.tr);
  const lead = o.impact?.lines.find((l) => l.lead) ?? null;
  const why = !o.available && o.unavailable_key ? tr.event(o.unavailable_key, o.unavailable_args) : null;
  const marks = [o.chosen ? tr.t("ui-proposal-option-chosen") : null, o.is_default && !o.chosen ? tr.t("ui-proposal-option-default") : null].filter(Boolean).join(" · ");
  return (
    <li className={o.chosen ? "pp-opt pp-opt-on" : o.available ? "pp-opt" : "pp-opt pp-opt-off"} data-testid={`proposal-option-${axis.id}-${o.id}`} data-chosen={o.chosen ? "true" : "false"} data-available={o.available ? "true" : "false"}>
      <label className="pp-opt-label">
        <input type="radio" name={name} value={o.id} checked={o.chosen} disabled={busy || (!o.available && !o.chosen)} onChange={onPick} />
        <span className="pp-opt-lines">
          <span className="pp-opt-l1">
            <strong>{tr.t(o.name_key)}</strong>
            {/* An option that cannot be combined is no package: no figures for it. */}
            {o.available && (
              <span className="num" data-testid="option-cost">
                {tr.t("ui-proposal-cost", { amount: formatMoney(o.est_annual_cost) })}
              </span>
            )}
          </span>
          <span className="pp-opt-l2 muted small">
            {(o.available ? [o.support ? supportText(tr, o.support) : null, tr.t("ui-impl-months", { n: o.months }), marks] : [tr.t("ui-proposal-unavailable"), marks]).filter(Boolean).join(" · ")}
          </span>
          {lead && (
            <span className="pp-opt-l3 small" data-testid="option-impact">
              {impactLine(tr, lead)}
            </span>
          )}
          <span className="pp-opt-desc small muted">{tr.t(o.desc_key)}</span>
          {why && (
            <span className="pp-opt-why small" data-testid="option-unavailable">
              {why}
            </span>
          )}
        </span>
      </label>
    </li>
  );
}

function Axis({ a, busy }: { a: AxisPreviewView; busy: boolean }) {
  const tr = useGame((s) => s.tr);
  const choose = useProposal((s) => s.choose);
  const name = useId();
  return (
    <fieldset className="pp-axis" data-testid={`proposal-axis-${a.id}`}>
      <legend className="sub">{tr.t(a.name_key)}</legend>
      <ul className="pp-opts">
        {a.options.map((o) => (
          <Option key={o.id} o={o} axis={a} name={name} busy={busy} onPick={() => void choose(a.id, o.id)} />
        ))}
      </ul>
    </fieldset>
  );
}

function Sponsors({ fam, amending }: { fam: FamilyPreviewView; amending: BillView | null }) {
  const tr = useGame((s) => s.tr);
  const setPackage = useProposal((s) => s.setPackage);
  const busy = useProposal((s) => s.open?.busy ?? false);
  if (fam.sponsors.length === 0) return null;
  return (
    <section className="pp-sec" data-testid="proposal-sponsors">
      <h3 className="sub">{tr.t("ui-proposal-sponsors")}</h3>
      <ul className="pp-sponsors">
        {fam.sponsors.map((s) => {
          const version = tr.event(s.name_key, fam.proposal.args);
          const who = whoText(tr, s);
          const dropped = amending?.family?.sponsor === s.id && !s.matches_current;
          return (
            <li key={s.id} className="pp-sponsor" data-testid={`proposal-sponsor-${s.id}`} data-matches={s.matches_current ? "true" : "false"}>
              <p>
                <strong>{tr.t("ui-proposal-sponsor-by", { version, who })}</strong>
              </p>
              <p className="small">{tr.t("ui-proposal-sponsor-terms", { terms: termsText(tr, fam, s) })}</p>
              <p className="small" data-testid="sponsor-price">
                {tr.t("ui-proposal-sponsor-price", { price: priceText(tr, s.price) })}
              </p>
              {dropped && (
                <p className="small pp-warn" data-testid="sponsor-drop">
                  {tr.t("ui-proposal-sponsor-drop", { version, who })}
                </p>
              )}
              {s.matches_current ? (
                <p className="chip pp-taken" data-testid="sponsor-taken">
                  {tr.t("ui-proposal-sponsor-taken")}
                </p>
              ) : (
                <button type="button" className="btn btn-sm" disabled={busy} aria-label={tr.t("ui-proposal-sponsor-take-label", { version })} onClick={() => void setPackage(s.preset, s.params)} data-testid="sponsor-take">
                  {tr.t("ui-proposal-sponsor-take")}
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** The package as set, in two lines at the top: its cost and time, and its support. */
function Package({ v, stale }: { v: PolicyPreviewView; stale: boolean }) {
  const tr = useGame((s) => s.tr);
  const fam = v.family;
  if (!fam) return null;
  return (
    <section className="pp-sec pp-package" data-testid="proposal-package" aria-busy={stale}>
      <h3 className="sub">{tr.t("ui-proposal-package")}</h3>
      <p className="small" data-testid="proposal-cost" aria-live="polite">
        <strong>{costLine(tr, v.est_annual_cost, v.budget_share, fam.months)}</strong>
        {stale && <span className="muted"> · {tr.t("ui-preview-updating")}</span>}
      </p>
      <p className="small" data-testid="proposal-support">
        {tr.t("ui-support")}: {supportText(tr, v.support)}
      </p>
    </section>
  );
}

/** What the package as set would do (the estimate's own lines, after the choices); "Why?" opens who backs it and what it costs people. */
function Effects({ v, stale }: { v: PolicyPreviewView; stale: boolean }) {
  const tr = useGame((s) => s.tr);
  const [why, setWhy] = useState(false);
  const fam = v.family;
  if (!fam) return null;
  return (
    <section className="pp-sec pp-effects" data-testid="proposal-effects" aria-busy={stale}>
      <ImpactLines impact={v.impact} />
      <button type="button" className="btn btn-sm btn-ghost pp-why-btn" aria-expanded={why} onClick={() => setWhy(!why)} data-testid="proposal-why">
        {tr.t("ui-why")}
      </button>
      {why && (
        <div className="why" data-testid="proposal-why-body">
          <p className="small muted">{tr.t("ui-proposal-why-help")}</p>
          {fam.proposal.reason_key && <p className="small">{tr.t("ui-proposal-why-now", { reason: tr.event(fam.proposal.reason_key, fam.proposal.args) })}</p>}
          <SupportBars s={v.support} />
          <TradeOffs effects={v.side_effects} />
        </div>
      )}
    </section>
  );
}

function DraftedBills({ p, onAmend }: { p: ProposalView; onAmend: (b: BillView) => void }) {
  const tr = useGame((s) => s.tr);
  const office = useGame((s) => s.office);
  const bills = p.bills.map((id) => office?.bills.find((b) => b.id === id)).filter((b): b is BillView => b !== undefined);
  if (bills.length === 0) return null;
  return (
    <section className="pp-sec" data-testid="proposal-bills">
      <h3 className="sub">{tr.t("ui-proposal-drafted")}</h3>
      <ul className="pp-bills">
        {bills.map((b) => {
          const editable = b.sponsor_is_player && (b.status === "draft" || b.status === "under_consideration");
          return (
            <li key={b.id} className="pp-bill" data-testid={`proposal-bill-${b.id}`} data-status={b.status}>
              <span>
                <strong>{billName(tr, b)}</strong>
                <span className="muted small"> · {tr.t(`status-${b.status}`)}</span>
              </span>
              {editable && (
                <button type="button" className="btn btn-sm" onClick={() => onAmend(b)} data-testid="proposal-bill-amend">
                  {tr.t("ui-amend-open")}
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** Drafting or amending, as the engine allows: why not, who could, and the command. */
function Command({ v, amending }: { v: PolicyPreviewView; amending: BillView | null }) {
  const tr = useGame((s) => s.tr);
  const gameBusy = useGame((s) => s.busy);
  const perform = useGame((s) => s.perform);
  const o = useProposal((s) => s.open);
  const markDone = useProposal((s) => s.markDone);
  const fam = v.family;
  if (!fam || !o) return null;
  const p = fam.proposal;
  const sponsor = fam.sponsors.find((s) => s.matches_current) ?? null;
  const unavailable = fam.unavailable_key ? tr.event(fam.unavailable_key, fam.unavailable_args) : null;
  const choices = fam.choices;
  const params = v.params;
  if (amending) {
    const pending = amending.status === "under_consideration";
    const disabled = gameBusy || o.busy || unavailable !== null;
    return (
      <section className="pp-sec pp-act" data-testid="proposal-command">
        {unavailable && <p className="small pp-warn">{unavailable}</p>}
        <p className="hint">{tr.t(pending ? "ui-amend-cost" : "ui-proposal-amend-help")}</p>
        <button
          type="button"
          className="btn btn-primary btn-block"
          disabled={disabled}
          data-testid="proposal-amend"
          onClick={() => {
            useGame.setState({ error: null });
            void perform({ type: "amend_policy", policy: amending.id, params, clauses: amending.clauses, choices }).then(() => {
              if (useGame.getState().error === null) markDone("amended", amending.id);
            });
          }}
        >
          {tr.t("ui-proposal-amend-save")}
        </button>
      </section>
    );
  }
  const blocked = p.blocked_key ? tr.t(p.blocked_key) : null;
  const disabled = gameBusy || o.busy || !p.can_draft || !v.available;
  return (
    <section className="pp-sec pp-act" data-testid="proposal-command">
      {unavailable && <p className="small pp-warn">{unavailable}</p>}
      {blocked && (
        <p className="small pp-warn" data-testid="proposal-blocked">
          {blocked}
          {p.blocked_key === "proposal-blocked-no-power" && p.proposers.length > 0 && <> {tr.t("ui-proposal-proposed-by", { posts: p.proposers.map((x) => postTitle(tr, x)).join(", ") })}</>}
        </p>
      )}
      {sponsor && <p className="small">{tr.t("ui-proposal-adopt-help")}</p>}
      <p className="hint">{tr.t("ui-proposal-draft-help")}</p>
      <button
        type="button"
        className="btn btn-primary btn-block"
        disabled={disabled}
        data-testid="proposal-draft"
        onClick={() => {
          void perform({ type: "draft_family", proposal: p.id, choices, params, sponsor: sponsor?.id ?? null }).then((id) => {
            if (id !== null) markDone("drafted", id);
          });
        }}
      >
        {tr.t(sponsor ? "ui-proposal-draft-sponsor" : "ui-proposal-draft")}
      </button>
    </section>
  );
}

/** Below "known": the title, the reason, who is behind it by role, and the lead (DEC-091). */
function Stub({ p }: { p: ProposalView }) {
  const tr = useGame((s) => s.tr);
  return (
    <div className="pp-stub" data-testid="proposal-stub">
      {p.reason_key && <p data-testid="proposal-reason">{tr.t("ui-proposal-why-now", { reason: tr.event(p.reason_key, p.args) })}</p>}
      <p className="small muted">{tr.t("ui-proposal-stub-help")}</p>
      {p.sponsors.length > 0 && (
        <ul className="plain small" data-testid="proposal-stub-sponsors">
          {p.sponsors.map((s) => (
            <li key={s.id}>{tr.t("ui-proposal-sponsor-by", { version: tr.event(s.name_key, p.args), who: whoText(tr, s) })}</li>
          ))}
        </ul>
      )}
      {p.blocked_key && <p className="small pp-warn">{tr.t(p.blocked_key)}</p>}
    </div>
  );
}

function Body({ v }: { v: PolicyPreviewView }) {
  const tr = useGame((s) => s.tr);
  const office = useGame((s) => s.office);
  const o = useProposal((s) => s.open);
  const setParams = useProposal((s) => s.setParams);
  const openProposal = useProposal((s) => s.openProposal);
  const leaveBill = useProposal((s) => s.leaveBill);
  const fam = v.family;
  if (!fam || !o) return null;
  const p = fam.proposal;
  const lead = p.knowledge.lead;
  if (p.level === "heard" || fam.axes.length === 0) {
    return (
      <>
        <Stub p={p} />
        {lead && <Lead lead={lead} />}
      </>
    );
  }
  const amending = o.bill !== null ? (office?.bills.find((b) => b.id === o.bill) ?? null) : null;
  const done: ReactNode = o.done ? (
    <p className="note pp-done" role="status" data-testid="proposal-result" data-kind={o.done.kind}>
      {tr.t(o.done.kind === "drafted" ? "ui-proposal-done" : "ui-proposal-amended")}
    </p>
  ) : null;
  return (
    <>
      {amending && (
        <div className="pp-amending" data-testid="proposal-amending">
          <p className="small">{tr.t("ui-proposal-amending", { name: billName(tr, amending) })}</p>
          <button type="button" className="btn btn-sm btn-ghost" onClick={() => void leaveBill()} data-testid="proposal-leave-bill">
            {tr.t("ui-proposal-back")}
          </button>
        </div>
      )}
      <p className="small">{tr.event(p.desc_key, p.args)}</p>
      {p.reason_key && (
        <p className="small muted" data-testid="proposal-reason">
          {tr.t("ui-proposal-why-now", { reason: tr.event(p.reason_key, p.args) })}
        </p>
      )}
      <Package v={v} stale={o.busy} />
      {fam.axes.map((a) => (
        <Axis key={a.id} a={a} busy={o.busy && o.view === null} />
      ))}
      {fam.param_defs.length > 0 && (
        <section className="pp-sec" data-testid="proposal-sliders">
          <h3 className="sub">{tr.t("ui-proposal-sliders")}</h3>
          <ParamSliders defs={fam.param_defs} values={o.params.length > 0 ? o.params : v.params} onChange={setParams} />
        </section>
      )}
      <Sponsors fam={fam} amending={amending} />
      <Effects v={v} stale={o.busy} />
      {done}
      <Command v={v} amending={amending} />
      <DraftedBills
        p={p}
        onAmend={(b) => void openProposal({ proposal: p.id, family: p.family, bill: b.id, choices: b.family?.choices ?? [], params: b.params })}
      />
      {lead && <Lead lead={lead} />}
    </>
  );
}

export function ProposalSheet() {
  const tr = useGame((s) => s.tr);
  const o = useProposal((s) => s.open);
  const close = useProposal((s) => s.close);
  const [closing, requestClose, sheetRef] = useSheetClose(close);
  const title = useRef<HTMLHeadingElement>(null);
  const key = o ? `${o.proposal}-${o.bill ?? ""}` : null;
  const office = useGame((s) => s.office);
  const runId = useGame((s) => s.run?.runId ?? null);
  const refresh = useProposal((s) => s.refresh);
  const shown = o !== null;
  // The desk changed (a command, a turn): the engine is asked again, so the sheet never shows an old package.
  const lastOffice = useRef(office);
  useEffect(() => {
    if (lastOffice.current !== office && shown) void refresh();
    lastOffice.current = office;
  }, [office, shown, refresh]);
  // Another run: the sheet belongs to the last one.
  const lastRun = useRef(runId);
  useEffect(() => {
    if (lastRun.current !== runId) useProposal.getState().close();
    lastRun.current = runId;
  }, [runId]);
  // Opening moves the keyboard to the title (the store gives it back on close).
  useEffect(() => {
    if (key === null) return;
    const raf = requestAnimationFrame(() => title.current?.focus());
    return () => cancelAnimationFrame(raf);
  }, [key]);
  if (!o) return null;
  const p = o.view?.family?.proposal ?? null;
  const head = p ? [tr.t(p.at.name_key, { name: p.at.name }), tr.t(`category-${p.category}`), p.status === "lapsed" ? tr.t("ui-proposal-lapsed") : p.level === "heard" ? tr.t("ui-proposal-heard") : null].filter(Boolean).join(" · ") : null;
  return (
    <>
      <div className={closing ? "sheet-scrim proposal-scrim sheet-closing" : "sheet-scrim proposal-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet proposal-sheet sheet-closing" : "sheet proposal-sheet"} aria-labelledby="proposal-title" data-testid="proposal-sheet" data-proposal={o.proposal} data-level={p?.level ?? ""}>
        <div className="sheet-head">
          <div>
            {head && <p className="muted small">{head}</p>}
            <h2 id="proposal-title" ref={title} tabIndex={-1}>
              {p ? proposalTitle(tr, p) : tr.t("ui-proposal-sheet")}
            </h2>
          </div>
          <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
        <div className="sheet-body pp-body">
          {o.view ? <Body v={o.view} /> : <p className="muted">{tr.t(o.gone ? "ui-proposal-gone" : "ui-proposal-loading")}</p>}
        </div>
      </aside>
    </>
  );
}
