// The career card as an image (M6 note 23: "Share" opens the device's share
// sheet with an image rendered on the device, from the view; nothing goes to
// a server). The same lines as the card on screen (`cardText.ts`), drawn on
// a canvas in the app's colours.
import type { CareerCardView } from "../../engine/protocol/CareerCardView";
import type { Translator } from "../../i18n/fluent";
import { arcLines, decisionLines, endedWhen, endingTitle, figureText, highestText, openingPlace, raceHead, raceOutcome, ratingText, TEXT_LIST_MAX, whoText, worldText, yearsText } from "./cardText";

const W = 1080;
const PAD = 72;
const INK = "#1d2230";
const MUTED = "#5b6273";
const ACCENT = "#2f5d8a";
const PAPER = "#f6f2e8";

type Line = { text: string; size: number; weight?: number; colour?: string; gap?: number };

function lines(tr: Translator, c: CareerCardView): Line[] {
  const out: Line[] = [];
  const head = (t: string) => out.push({ text: t.toUpperCase(), size: 26, weight: 700, colour: ACCENT, gap: 28 });
  const body = (t: string, colour = INK) => out.push({ text: t, size: 32, colour });
  out.push({ text: tr.t(c.unfinished ? "ui-card-text-title-unfinished" : "ui-card-text-title"), size: 30, weight: 700, colour: ACCENT });
  out.push({ text: c.name, size: 60, weight: 700, gap: 16 });
  body(whoText(tr, c), MUTED);
  body(worldText(tr, c), MUTED);
  head(tr.t("ui-card-opening"));
  body(openingPlace(tr, c));
  body(ratingText(tr, c.opening.rating), MUTED);
  head(tr.t("ui-card-races"));
  if (c.races.length === 0) body(tr.t("ui-card-races-none"), MUTED);
  for (const r of c.races) {
    body(`${raceHead(tr, r)}: ${raceOutcome(tr, r)}`);
    body(ratingText(tr, r.rating), MUTED);
  }
  head(tr.t("ui-card-highest"));
  body(`${highestText(tr, c)} · ${yearsText(tr, c)}`);
  const d = decisionLines(tr, c);
  if (d.length) {
    head(tr.t("ui-card-decisions"));
    for (const x of d.slice(0, TEXT_LIST_MAX)) body(x);
    if (d.length > TEXT_LIST_MAX) body(tr.t("ui-card-more", { n: d.length - TEXT_LIST_MAX }), MUTED);
  }
  const a = arcLines(tr, c);
  if (a.length) {
    head(tr.t("ui-card-arcs"));
    for (const x of a.slice(0, TEXT_LIST_MAX)) body(x);
    if (a.length > TEXT_LIST_MAX) body(tr.t("ui-card-more", { n: a.length - TEXT_LIST_MAX }), MUTED);
  }
  head(tr.t("ui-card-ended"));
  body(`${endingTitle(tr, c)}, ${endedWhen(tr, c)}`);
  head(tr.t("ui-card-figure"));
  out.push({ text: figureText(tr, c), size: 40, weight: 700 });
  head(tr.t("ui-card-code"));
  out.push({ text: c.code ?? tr.t("ui-card-code-none"), size: 30, weight: 600, colour: INK });
  body(tr.t("ui-card-build", { build: c.build }), MUTED);
  return out;
}

/** Break a line to the width, by words. */
function wrap(ctx: CanvasRenderingContext2D, text: string, width: number): string[] {
  const words = text.split(/\s+/);
  const out: string[] = [];
  let line = "";
  for (const w of words) {
    const next = line ? `${line} ${w}` : w;
    if (ctx.measureText(next).width > width && line) {
      out.push(line);
      line = w;
    } else line = next;
  }
  if (line) out.push(line);
  return out;
}

const font = (l: Line) => `${l.weight ?? 400} ${l.size}px system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`;

/** The card drawn as a PNG. Rejects where the device has no canvas. */
export async function cardImage(tr: Translator, c: CareerCardView): Promise<Blob> {
  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no canvas");
  const all = lines(tr, c);
  // Lay out first, to size the canvas.
  const laid: { l: Line; rows: string[] }[] = [];
  let h = PAD;
  for (const l of all) {
    ctx.font = font(l);
    const rows = wrap(ctx, l.text, W - 2 * PAD);
    laid.push({ l, rows });
    h += (l.gap ?? 6) + rows.length * l.size * 1.3;
  }
  h += PAD;
  canvas.width = W;
  canvas.height = Math.ceil(h);
  ctx.fillStyle = PAPER;
  ctx.fillRect(0, 0, W, canvas.height);
  ctx.fillStyle = ACCENT;
  ctx.fillRect(0, 0, W, 16);
  let y = PAD;
  ctx.textBaseline = "top";
  for (const { l, rows } of laid) {
    y += l.gap ?? 6;
    ctx.font = font(l);
    ctx.fillStyle = l.colour ?? INK;
    for (const r of rows) {
      ctx.fillText(r, PAD, y);
      y += l.size * 1.3;
    }
  }
  return new Promise((resolve, reject) => canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("no image"))), "image/png"));
}
