// The pocket's People tab (DEC-088 main screen): the people who matter in town
// and how they stand with you, from the views the engine already gives (the
// council and mayor from the office view, the power broker and the rival from
// the town's character), each as far as the player knows them (DEC-091).
// Tapping a name opens their character sheet (DEC-031).
//
// Seam for the notebook (K5): the notebook page ("what you know", with leads)
// joins this tab. Until then the tab says how much was learned this turn
// (`PocketView.notebook_new`, also the tab's badge); K5 replaces
// `NotebookSeam` with the notebook itself (`QueryDto` `notebook`).
import { useGame } from "../state/gameStore";
import { Member } from "./Chamber";
import { Who } from "./TownCard";

function NotebookSeam({ n }: { n: number }) {
  const tr = useGame((s) => s.tr);
  if (n <= 0) return null;
  return (
    <p className="note" data-testid="people-notebook">
      {tr.t("ui-people-learned", { n })}
    </p>
  );
}

export function PeopleList() {
  const tr = useGame((s) => s.tr);
  const office = useGame((s) => s.office);
  const town = useGame((s) => s.town);
  const learned = useGame((s) => s.pocket?.notebook_new ?? 0);
  if (!office) return null;
  const council = office.council.filter((m) => !m.is_player);
  const mayor = office.mayor && !office.mayor.is_player && !office.council.some((m) => m.id === office.mayor?.id) ? office.mayor : null;
  const c = town?.character ?? null;
  return (
    <>
      <NotebookSeam n={learned} />
      {c && (c.broker || c.rival) && (
        <section data-testid="people-town" aria-labelledby="people-town-title">
          <h3 className="sub" id="people-town-title">
            {tr.t("ui-people-town")}
          </h3>
          <ul className="plain-list people-figures">
            {c.broker && (
              <li>
                <span className="muted small">{tr.t("ui-power-broker-line")}: </span>
                <Who p={c.broker} />
              </li>
            )}
            {c.rival && (
              <li>
                <span className="muted small">{tr.t("ui-rival-line")}: </span>
                <Who p={c.rival} />
              </li>
            )}
          </ul>
        </section>
      )}
      <section data-testid="people-council" aria-labelledby="people-council-title">
        <h3 className="sub" id="people-council-title">
          {tr.t("ui-council")}
        </h3>
        <ul className="member-list">
          {mayor && <Member m={mayor} />}
          {council.map((m) => (
            <Member key={m.id} m={m} />
          ))}
        </ul>
      </section>
      <p className="hint">{tr.t("ui-people-help")}</p>
    </>
  );
}
