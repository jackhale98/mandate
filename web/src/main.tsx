import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";

// The content workbench (dev only): `?dev=1` under `npm run dev`, or in a
// `vite build --mode devtools` bundle. Both flags are build-time constants,
// so a release build drops the page and this branch entirely.
const DEV_TOOLS = import.meta.env.DEV || import.meta.env.MODE === "devtools";

const root = document.getElementById("root");
if (root) {
  if (DEV_TOOLS && new URLSearchParams(location.search).get("dev") === "1") {
    void import("./dev/DesignerPage").then(({ DesignerPage }) =>
      createRoot(root).render(
        <StrictMode>
          <DesignerPage />
        </StrictMode>,
      ),
    );
  } else {
    createRoot(root).render(
      <StrictMode>
        <App />
      </StrictMode>,
    );
  }
}
