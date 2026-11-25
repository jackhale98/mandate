// A bill as a row (DEC-113 §3): its name, where it stands in a few words, the
// next vote; opened, the engine's count, its note, and a way to where it is
// moved (drafted, voted on). The impact estimate's four lines join it when
// that sheet is built (IMPACT_ESTIMATE §6).
import type { PocketBillView } from "../../engine/protocol/PocketBillView";
import type { PocketView } from "../../engine/protocol/PocketView";
import { formatDate } from "../../i18n/format";
import { placeIcon, placeName } from "../../places/labels";
import { useGame } from "../../state/gameStore";
import { routeFor } from "../../state/routing";
import { Row, When, shortDate, useFold } from "./rows";

export function BillRow({ b, p, testId }: { b: PocketBillView; p: PocketView; testId: string }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const [open, setOpen] = useFold(`row-${testId}`, false);
  const route = routeFor(p, { action: b.status === "draft" ? "draft" : "vote", lateNight: false, neighborhood: null });
  const name = tr.t(b.name_key);
  const status = [tr.t(`status-${b.status}`)];
  if (b.status === "under_consideration") status.push(tr.t("ui-whip-about", { yes: b.about_yes, seats: b.seats, needed: b.needed }));
  if (b.can_vote) status.push(tr.t("ui-awaits-your-vote"));
  return (
    <Row
      mark="📜"
      title={name}
      role={b.sponsor_is_player ? tr.t("ui-bill-yours") : undefined}
      sub={status.join(" · ")}
      end={b.next_vote ? <When top={shortDate(tr, b.next_vote)} under={b.next_stage_key ? tr.t(b.next_stage_key) : null} /> : undefined}
      open={open}
      onToggle={() => setOpen(!open)}
      li={{ "data-testid": testId, "data-bill": String(b.id) }}
    >
      {b.next_vote && b.next_stage_key && <p>{tr.t("ui-next-vote", { stage: tr.t(b.next_stage_key), date: formatDate(tr, b.next_vote) })}</p>}
      {b.note && <p className="rw-know">{tr.event(b.note.key, b.note.args)}</p>}
      {route?.to && (
        <div className="rw-acts">
          <button type="button" className="btn btn-sm" disabled={busy} onClick={() => route.to && void visit(route.to.location)}>
            <span aria-hidden="true">{placeIcon(route.to.kind_key)} </span>
            {placeName(tr, route.to)}
          </button>
        </div>
      )}
    </Row>
  );
}
