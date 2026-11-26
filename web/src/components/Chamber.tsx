import { useState, type ReactNode } from "react";
import type { MemberView } from "../engine/protocol/MemberView";
import type { BillView } from "../engine/protocol/BillView";
import type { VoteDto } from "../engine/protocol/VoteDto";
import { useGame } from "../state/gameStore";
import { formatDate } from "../i18n/format";
import { ImpactLines } from "./BillCard";
import { personName } from "./Knowing";
import { legislature, signerText } from "./posts";
import { useTalkFocus } from "./talk/focus";

/** Someone in office: name (opens their sheet), role, stances as known, and how they stand with you. */
export function Member({ m, children }: { m: MemberView; children?: ReactNode }) {
  const tr = useGame((s) => s.tr);
  const openCharacter = useGame((s) => s.openCharacter);
  return (
    <li className="member">
      <div>
        <button type="button" className="link-btn" onClick={() => void openCharacter(m.character)} data-testid="member-name">
          <strong>{personName(tr, m.name)}</strong>
        </button>
        {m.is_player && <span className="chip chip-muted"> {tr.t("ui-you")}</span>}
        <div className="muted small">
          {tr.t(m.role_key)}
          {m.ward_number !== null && <> · {tr.t("ui-ward", { n: m.ward_number })}</>}
          {m.pragmatism !== null && <> · {tr.t(m.pragmatism >= 0.55 ? "ui-pragmatic" : "ui-ideologue")}</>}
        </div>
        <div className="tags">
          {m.convictions.length === 0 && !m.is_player && <span className="muted small">{tr.t("knowledge-stances-unknown")}</span>}
          {m.convictions.map((c) => (
            <span key={c} className="chip chip-muted">
              {tr.t(`conviction-${c}`)}
            </span>
          ))}
        </div>
      </div>
      {!m.is_player && (
        <div className="member-side small">
          <span>{tr.t("ui-opinion-of-you", { n: Math.round(m.opinion_of_player) })}</span>
          <span className="muted">{m.favors >= 0 ? tr.t("ui-favors", { n: m.favors }) : tr.t("ui-owe", { n: -m.favors })}</span>
          {children}
        </div>
      )}
    </li>
  );
}

/** A reason's args: the `issue` is itself a Fluent key. */
function issueArgs(tr: { t: (k: string, a?: Record<string, string | number>) => string }, args: Record<string, string>): Record<string, string> {
  const out: Record<string, string> = { ...args };
  if (args.issue) out.issue = tr.t(args.issue);
  return out;
}

/** The player's own vote: public, final, and weighed against party, ward, and favours (DEC-073). */
export function YourVote({ bill }: { bill: BillView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const perform = useGame((s) => s.perform);
  if (!bill.can_vote && !bill.my_vote) return null;
  return (
    <div className="your-vote" data-testid="your-vote">
      <h4 className="sub">{tr.t("ui-your-vote")}</h4>
      <ul className="plain small">
        <li>
          {tr.t(`ui-party-line-${bill.party_line}`)}
          {bill.party_line_reason && (
            <span className="hint" data-testid="party-line-reason">
              {" "}
              {tr.t(bill.party_line_reason.key, issueArgs(tr, bill.party_line_reason.args))}
            </span>
          )}
        </li>
        <li data-testid="ward-view">
          {tr.t("ui-ward-view", {
            lean: tr.t(`ward-lean-${bill.ward_lean}`),
            for: bill.ward_for_pct,
            against: bill.ward_against_pct,
            certainty: tr.t(`ward-certainty-${bill.ward_certainty}`),
          })}
        </li>
        {bill.owe_sponsor && <li>{tr.t("ui-owe-sponsor", { name: personName(tr, bill.sponsor_name) })}</li>}
      </ul>
      {bill.my_vote ? (
        <p className="chip">{tr.t(`ui-voted-${bill.my_vote}`)}</p>
      ) : (
        <>
          <div className="bill-actions">
            {(["yes", "no", "abstain"] as VoteDto[]).map((v) => (
              <button key={v} type="button" className={v === "yes" ? "btn btn-sm btn-primary" : "btn btn-sm"} disabled={busy} onClick={() => void perform({ type: "cast_vote", policy: bill.id, vote: v })}>
                {tr.t(`ui-vote-${v}`)}
              </button>
            ))}
          </div>
          <p className="hint">{tr.t("ui-vote-help")}</p>
        </>
      )}
    </div>
  );
}

/** A member's reasons on the whip, as the engine states them (a `*_key` argument is itself a key). */
function reasonText(tr: { event: (k: string, a: Record<string, string>) => string }, r: { key: string; args: Record<string, string> }): string {
  return tr.event(r.key, r.args);
}

/**
 * The whip count, member by member (in the legislature's chamber only, DEC-079): for, against,
 * undecided, and why as far as the player knows. Lobbying is a conversation (M6 note 29): each row
 * offers "Talk to them", which opens it on the bill without a walk (DEC-051).
 */
export function ChamberWhip() {
  const tr = useGame((s) => s.tr);
  const o = useGame((s) => s.office);
  const busy = useGame((s) => s.busy);
  const openTalk = useGame((s) => s.openTalk);
  const pending = (o?.bills ?? []).filter((b) => b.status === "under_consideration");
  const [sel, setSel] = useState<number | null>(null);
  if (!o) return null;
  const bill = pending.find((b) => b.id === sel) ?? pending[0];
  const body = legislature(o);
  const talk = (character: number, policy: number) => {
    useTalkFocus.getState().setBill(policy);
    void openTalk({ type: "person", person: character });
  };

  return (
    <>
      <section className="card" data-testid="whip">
        <div className="card-head">
          <h2>{body ? tr.t(body.label_key) : tr.t("ui-members")}</h2>
        </div>
        {!bill ? (
          <p className="muted">{tr.t("ui-no-pending")}</p>
        ) : (
          <>
            {pending.length > 1 && (
              <label className="field">
                <span>{tr.t("ui-select-bill")}</span>
                <select className="input" value={bill.id} onChange={(e) => setSel(Number(e.target.value))}>
                  {pending.map((b) => (
                    <option key={b.id} value={b.id}>
                      {tr.t(b.name_key)}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <h3 className="bill-title">{tr.t(bill.name_key)}</h3>
            <ImpactLines impact={bill.impact} />
            {bill.next_vote && bill.next_stage_key && (
              <p className="small">{tr.t("ui-next-vote", { stage: tr.t(bill.next_stage_key), date: formatDate(tr, bill.next_vote) })}</p>
            )}
            <p className="small">
              <strong>{bill.whip_exact ? tr.t("ui-whip", { yes: bill.expected_yes.toFixed(1), seats: bill.seats, needed: bill.needed }) : tr.t("ui-whip-about", { yes: bill.expected_yes, seats: bill.seats, needed: bill.needed })}</strong>
              {bill.signer && <span data-testid="signer-lean"> · {signerText(tr, bill.signer)}</span>}
            </p>
            <YourVote bill={bill} />
            <p className="hint">{tr.t("ui-whip-help")}</p>
            <ul className="whip-list">
              {bill.whip.map((w) => {
                const who = personName(tr, w.name);
                return (
                  <li key={w.politician} className="whip-row" data-lean={w.lean} data-settled={w.settled ? "true" : undefined} data-testid="whip-row">
                    <div className="whip-who">
                      <strong>{who}</strong>
                      <span className="muted small">
                        {tr.t("ui-ward", { n: w.ward_number })} · <span className={`lean lean-${w.lean}`}>{tr.t(`lean-${w.lean}`)}</span> · {tr.t("ui-yes-chance", { pct: Math.round(w.yes_probability * 100) })}
                      </span>
                      {w.reasons.length > 0 && (
                        <span className="small whip-why" data-testid="whip-why">
                          {w.reasons.map((r) => reasonText(tr, r)).join(" · ")}
                        </span>
                      )}
                    </div>
                    {w.is_player ? (
                      <span className="chip chip-muted">{tr.t("ui-you")}</span>
                    ) : w.settled ? (
                      <span className="chip chip-muted" data-testid="whip-settled">
                        {tr.t("ui-whip-settled")}
                      </span>
                    ) : (
                      <div className="whip-actions">
                        {w.brings_others && <span className="chip">{tr.t("ui-whip-brings")}</span>}
                        <button
                          type="button"
                          className="btn btn-sm"
                          disabled={busy || w.exchanges_left === 0}
                          aria-label={tr.t("ui-whip-talk-to", { name: who })}
                          onClick={() => talk(w.character, bill.id)}
                          data-testid="whip-talk"
                        >
                          {tr.t("ui-whip-talk")}
                        </button>
                        {w.exchanges_left === 0 && <span className="muted small">{tr.t("ui-whip-talked-out")}</span>}
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </>
        )}
      </section>
      <section className="card" data-testid="members">
        <h2>{tr.t("ui-members")}</h2>
        <ul className="member-list">
          {o.executive && <Member m={o.executive} />}
          {o.bodies.flatMap((b) =>
            b.members.map((m) => (
              <Member key={`${b.id}-${m.id}`} m={m} />
            )),
          )}
        </ul>
      </section>
    </>
  );
}
