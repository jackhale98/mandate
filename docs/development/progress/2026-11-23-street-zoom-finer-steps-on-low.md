# Street zoom: finer steps on low-DPR screens

**Date:** 2026-11-23
**Status:** done

Web only. The street view settled on whole device pixels, so a DPR 1 desktop had six zoom states. `zoomStepFor(dpr)` in `web/src/hub/hubView.ts` now subdivides the step to at most 0.25 CSS px per art px when 1 / dpr is above 0.5 (DPR 1: 0.25, 21 states from 1 to 6; 1.25: 0.2; 1.5: 0.222); DPR 2 and up are unchanged (DPR 2: 12 states). The lowest zoom stays one device pixel per art pixel and the maximum is 6; first-open and conversation framing use the same step. Checked at DPR 1 at zooms 1.25, 2.25, 2.5, 2.75 and 3.75 while panning: no tile seams, so no offscreen pass was added and the art draws on the same direct path. Wheel and pinch needed no change: a notch moves at least one step and under a quarter of the zoom. The street has no keyboard zoom (+ and -); none was added.
