// A hotspot's sheet (DEC-088): what the recipe puts at the counter, the
// benches or a booth, filtered by what the engine's place view offers now,
// through the forms and flows the place panel uses (DEC-079: the actions
// cost and route as anywhere else). Services open the place panel, where
// their figures are. Nothing here decides an outcome.
import { useRef } from "react";
import type { PlacePanelView } from "../engine/protocol/PlacePanelView";
import type { HotspotDef } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";
import { useGame } from "../state/gameStore";
import { VerbTalk } from "../components/talk/VerbTalk";
import { ChamberWhip } from "../components/Chamber";
import { Catalog, MyBills } from "../components/Desk";
import { Calendar } from "../components/CampaignCards";
import { useSheetClose } from "../components/useSheetClose";
import { actionKey } from "../places/labels";

/** What a hotspot offers now: its actions the place view lists (late ones only as late-night offers) and its services. */
export function hotspotOffers(h: HotspotDef, v: Pick<PlacePanelView, "actions" | "services">): { actions: string[]; services: string[] } {
  return {
    actions: (h.actions ?? []).filter((a) => v.actions.some((o) => o.action === a && (!h.late || o.late_night))),
    services: (h.services ?? []).filter((s) => v.services.includes(s)),
  };
}

// Lobbying is a conversation (M6 note 29): a booth says so. The envelope is
// had with whoever is offering, in the booth (DEC-114, B5).
const FORMS = new Set<string>(["take_bribe"]);
const TALKED = new Set<string>(["lobby", "trade_favor"]);

/** A verb's offer here, its late variant first (the envelope is late). */
const offerOf = (v: PlacePanelView, a: string) => v.actions.find((o) => o.action === a && o.late_night) ?? v.actions.find((o) => o.action === a);

export function HotspotSheet({ hotspot, view, onClose }: { hotspot: HotspotDef; view: PlacePanelView; onClose: () => void }) {
  const tr = useGame((s) => s.tr);
  const campaign = useGame((s) => s.campaign);
  const openPanel = useGame((s) => s.openPanel);
  const ref = useRef<HTMLElement>(null);
  const [closing, requestClose, sheetRef] = useSheetClose(onClose, ref);
  const { actions, services } = hotspotOffers(hotspot, view);
  const has = (a: string) => actions.includes(a);
  // The player's bills at the stages this hotspot handles.
  const statuses = [...new Set([...(has("introduce") || has("withdraw") || has("draft") ? ["draft"] : []), ...(has("amend") || has("withdraw") ? ["under_consideration"] : [])])];
  const whip = has("vote");
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet place-panel sheet-closing" : "sheet place-panel"} aria-labelledby="hotspot-title" data-testid="interior-hotspot" data-hotspot={hotspot.id}>
        <div className="sheet-head">
          <div>
            <h2 id="hotspot-title">{tr.t(`interior-hotspot-${hotspot.id}`)}</h2>
            <p className="muted small">{view.location.name || tr.t(`place-${view.location.kind_key}`)}</p>
          </div>
          <div className="sheet-tools">
            <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
              ✕
            </button>
          </div>
        </div>
        <div className="sheet-body place-body">
          {actions.length + services.length === 0 && <p className="muted">{tr.t("ui-interior-nothing")}</p>}
          {whip && <ChamberWhip />}
          {statuses.length > 0 && <MyBills statuses={statuses} testId="hotspot-bills" />}
          {has("draft") && <Catalog />}
          {has("file") && campaign && <Calendar c={campaign} />}
          {actions
            .filter((a) => FORMS.has(a) && !whip)
            .map((a) => (
              <section key={a} className="card" data-testid={`hotspot-action-${a}`}>
                <h2>{tr.t(actionKey(a))}</h2>
                <VerbTalk action={a} person={offerOf(view, a)?.with} to={view.location} here={view.here} topic={offerOf(view, a)?.topic ?? null} />
              </section>
            ))}
          {!whip && actions.some((a) => TALKED.has(a)) && (
            <p className="muted small" data-testid="hotspot-lobby-talk">
              {tr.t("ui-lobby-by-talking")}
            </p>
          )}
          {services.map((s) => (
            <section key={s} className="card" data-testid={`hotspot-service-${s}`}>
              <h2>{tr.t(`service-${s}`)}</h2>
              <p className="muted small">{tr.t("ui-interior-read-note")}</p>
              <button
                type="button"
                className="btn"
                onClick={() => {
                  onClose();
                  void openPanel(view.location.location);
                }}
              >
                {tr.t("ui-interior-read")}
              </button>
            </section>
          ))}
        </div>
      </aside>
    </>
  );
}
