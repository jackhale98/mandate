import { useEffect } from "react";
import { useGame } from "./state/gameStore";
import { Title } from "./screens/Title";
import { Game } from "./screens/Game";
import { ErrorBanner } from "./components/ErrorBanner";
import { UpdatePrompt } from "./components/UpdatePrompt";

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
        <main className="center" aria-busy="true">
          <div className="spinner" aria-hidden="true" />
          <p>{tr.has("ui-loading-engine") ? tr.t("ui-loading-engine") : "Starting the engine…"}</p>
        </main>
      ) : screen === "title" ? (
        <Title />
      ) : (
        <Game />
      )}
      <UpdatePrompt />
    </div>
  );
}
