import { useState } from "react";
import type { MemberView } from "../engine/protocol/MemberView";
import { useGame } from "../state/gameStore";
import { formatDate } from "../i18n/format";

function Member({ m }: { m: MemberView }) {
  const tr = useGame((s) => s.tr);
  return (
    <li className="member">
      <div>
        <strong>{m.name}</strong>
        {m.is_player && <span className="chip chip-muted"> {tr.t("ui-you")}</span>}
        <div className="muted small">
          {tr.t(m.role_key)}
          {m.ward_number !== null && <> · {tr.t("ui-ward", { n: m.ward_number })}</>} · {tr.t(m.pragmatism >= 0.55 ? "ui-pragmatic" : "ui-ideologue")}
        </div>
        <div className="tags">
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
        </div>
      )}
    </li>
  );
}

export function Council() {
  const tr = useGame((s) => s.tr);
  const o = useGame((s) => s.office);
  const busy = useGame((s) => s.busy);
  const command = useGame((s) => s.command);
  const pending = (o?.bills ?? []).filter((b) => b.status === "under_consideration");
  const [sel, setSel] = useState<number | null>(null);
  if (!o) return null;
  const bill = pending.find((b) => b.id === sel) ?? pending[0];
  const favours = new Map(o.council.map((m) => [m.id, m.favors]));
  const p = o.player;

  return (
    <main className="game-body">
      <section className="card" data-testid="whip">
        <div className="card-head">
          <h2>{tr.t("ui-council")}</h2>
          <span className="chip">{tr.t("ui-slots", { left: p.slots_left })}</span>
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
            {bill.next_vote && bill.next_stage_key && (
              <p className="small">{tr.t("ui-next-vote", { stage: tr.t(bill.next_stage_key), date: formatDate(tr, bill.next_vote) })}</p>
            )}
            <p className="small">
              <strong>{tr.t("ui-whip", { yes: bill.expected_yes.toFixed(1), seats: bill.seats, needed: bill.needed })}</strong> ·{" "}
              {tr.t("ui-mayor-lean", { lean: tr.t(`mayor-${bill.mayor}`) })}
            </p>
            <p className="hint">{tr.t("ui-whip-help")}</p>
            <ul className="whip-list">
              {bill.whip.map((w) => {
                const owed = favours.get(w.politician) ?? 0;
                return (
                  <li key={w.politician} className="whip-row" data-lean={w.lean}>
                    <div className="whip-who">
                      <strong>{w.name}</strong>
                      <span className="muted small">
                        {tr.t("ui-ward", { n: w.ward_number })} · <span className={`lean lean-${w.lean}`}>{tr.t(`lean-${w.lean}`)}</span> · {tr.t("ui-yes-chance", { pct: Math.round(w.yes_probability * 100) })}
                      </span>
                    </div>
                    {w.is_player ? (
                      <span className="chip chip-muted">{tr.t("ui-you")}</span>
                    ) : (
                      <div className="whip-actions">
                        <button
                          type="button"
                          className="btn btn-sm"
                          disabled={busy || p.slots_left === 0}
                          onClick={() => void command({ type: "act", action: { type: "lobby", politician: w.politician, policy: bill.id }, late_night: false })}
                        >
                          {tr.t("action-lobby")} · {tr.t("ui-odds", { pct: Math.round(w.lobby_odds * 100) })}
                        </button>
                        <button
                          type="button"
                          className="btn btn-sm btn-ghost"
                          disabled={busy || !p.late_night_available}
                          onClick={() => void command({ type: "act", action: { type: "lobby", politician: w.politician, policy: bill.id }, late_night: true })}
                        >
                          {tr.t("ui-late-night-short")} · {tr.t("ui-odds", { pct: Math.round(w.lobby_odds_late * 100) })}
                        </button>
                        {owed > -2 && (
                          <button
                            type="button"
                            className="btn btn-sm btn-ghost"
                            disabled={busy || p.slots_left === 0}
                            onClick={() => void command({ type: "act", action: { type: "trade_favor", politician: w.politician, policy: bill.id }, late_night: false })}
                          >
                            {tr.t("action-trade-favor")}
                          </button>
                        )}
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </>
        )}
      </section>
      <section className="card">
        <h2>{tr.t("ui-members")}</h2>
        <ul className="member-list">
          {o.mayor && <Member m={o.mayor} />}
          {o.council.map((m) => (
            <Member key={m.id} m={m} />
          ))}
        </ul>
      </section>
    </main>
  );
}
