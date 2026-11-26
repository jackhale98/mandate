import { useEffect } from "react";
import { useGame } from "./state/gameStore";
import { Title } from "./screens/Title";
import { Game } from "./screens/Game";
import { Draft } from "./screens/Draft";
import { ErrorBanner } from "./components/ErrorBanner";
import { UpdatePrompt } from "./components/UpdatePrompt";
import { Loader } from "./components/Brand";
import { ReportDialog } from "./components/report/ReportDialog";

export function App() {
  const screen = useGame((s) => s.screen);
  const booting = useGame((s) => s.booting);
  const tr = useGame((s) => s.tr);
  const boot = useGame((s) => s.boot);

  useEffect(() => {
    void boot();
  }, [boot]);

  return (
    <div className="app">
      <ErrorBanner />
      {booting ? (
        <main className="center">
          <Loader label={tr.has("ui-loading-engine") ? tr.t("ui-loading-engine") : "Starting the engine…"} />
        </main>
      ) : screen === "title" ? (
        <Title />
      ) : screen === "draft" ? (
        <Draft />
      ) : (
        <Game />
      )}
      <UpdatePrompt />
      <ReportDialog />
    </div>
  );
}
