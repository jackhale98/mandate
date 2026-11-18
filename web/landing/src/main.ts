// The landing page's only script (DEC-083). The page works without it: the
// call to action is a plain link. This adds two things:
//  1. "Continue your run" when this browser holds a saved run (read-only);
//  2. the results tally and the ladder animate once when scrolled into view.
import "./styles.css";

import { describe, latestRun } from "./runs";

async function showContinue() {
  const slot = document.querySelector<HTMLElement>("[data-continue]");
  if (!slot) return;
  const run = await latestRun().catch(() => null);
  if (!run) return;
  const link = slot.querySelector<HTMLAnchorElement>("a");
  const detail = slot.querySelector<HTMLElement>("[data-continue-detail]");
  if (!link || !detail) return;
  detail.textContent = describe(run);
  slot.hidden = false;
  document.querySelector<HTMLElement>("[data-free-line]")?.setAttribute("hidden", "");
}

function revealOnScroll() {
  const targets = document.querySelectorAll<HTMLElement>("[data-reveal]");
  if (!("IntersectionObserver" in window)) {
    targets.forEach((t) => t.classList.add("in-view"));
    return;
  }
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        e.target.classList.add("in-view");
        io.unobserve(e.target);
      }
    },
    { rootMargin: "0px 0px -15% 0px" },
  );
  targets.forEach((t) => io.observe(t));
}

revealOnScroll();
void showContinue();
