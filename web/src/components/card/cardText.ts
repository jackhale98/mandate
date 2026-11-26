// The career card's words (M6 note 23): the engine's `CareerCardView` (facts
// and Fluent keys) put into lines, for the card on screen, the text "Copy"
// gives and the image "Share" draws. Every figure is the engine's; this file
// only formats them (a thousandth as a percentage, a share of the margin as
// points, tenths of a year).
import type { CareerCardView } from "../../engine/protocol/CareerCardView";
import type { CardRaceView } from "../../engine/protocol/CardRaceView";
import type { PostDto } from "../../engine/protocol/PostDto";
import type { RatingView } from "../../engine/protocol/RatingView";
import type { Translator } from "../../i18n/fluent";
import { formatDate } from "../../i18n/format";
import { actArgs } from "../act/actArgs";

/** Lines of the copied text the lists are cut to (the card on screen shows them all). */
export const TEXT_LIST_MAX = 3;

/** A thousandth as a whole percentage. */
export const pct = (pm: number): string => `${Math.round(pm / 10)}%`;

/** A signed number of thousandths as a decimal (`+0.25`, `−1.50`, `0.00`). */
export function signed(pm: number): string {
  const v = (Math.abs(pm) / 1000).toFixed(2);
  if (pm > 0) return `+${v}`;
  if (pm < 0) return `−${v}`;
  return v;
}

/** A factor's share of the margin (thousandths of the vote) as signed points. */
export function points(pm: number): string {
  const v = Math.round(Math.abs(pm) / 10);
  if (v === 0) return "0";
  return pm > 0 ? `+${v}` : `−${v}`;
}

/** A post as its holder's title (the constitution's role word) and its place. */
export function postName(tr: Translator, p: PostDto): string {
  return tr.t("ui-card-highest-post", { post: tr.t(p.role_key), place: p.at.name || tr.t(p.at.name_key, { name: p.at.name }) });
}

/** A rating: its win rate and plays, and the factors behind it. */
export function ratingText(tr: Translator, r: RatingView | null): string {
  if (!r) return tr.t("ui-card-rating-none");
  const head = tr.t("ui-card-rating", { pct: pct(r.win_pm), wins: r.wins, plays: r.plays });
  const facts = r.facts.map((f) => tr.t("ui-card-fact", { factor: tr.t(f.key), margin: points(f.margin_pm) }));
  return facts.length ? `${head}: ${facts.join(", ")}` : head;
}

export function whoText(tr: Translator, c: CareerCardView): string {
  return c.party ? tr.t("ui-card-who", { name: c.name, party: c.party, town: c.town }) : tr.t("ui-card-who-independent", { name: c.name, town: c.town });
}

export function worldText(tr: Translator, c: CareerCardView): string {
  return tr.t("ui-card-world", {
    size: tr.t(`ui-card-size-${c.world.size}`),
    scenario: c.world.scenario_key ? tr.t(c.world.scenario_key) : tr.t("ui-card-world-random"),
    constitution: tr.t(c.world.constitution_key),
  });
}

/** Where and on what play began, and how far on. */
export function openingPlace(tr: Translator, c: CareerCardView): string {
  const o = c.opening;
  const place = o.arc ? tr.t("ui-card-opening-arc", { arc: tr.event(o.arc.key, o.arc.args), town: o.town }) : tr.t("ui-card-opening-plain", { town: o.town });
  return `${place}, ${tr.t("ui-card-opening-ahead", { n: o.months_ahead })}`;
}

export function raceHead(tr: Translator, r: CardRaceView): string {
  // The year as written, not as a grouped number.
  return tr.t("ui-card-race", { year: String(r.election_day.year), post: tr.t(r.post.role_key), place: r.post.at.name });
}

export function raceOutcome(tr: Translator, r: CardRaceView): string {
  const share = r.share_pm === null ? "" : pct(r.share_pm);
  return tr.t(`ui-card-race-${r.outcome}`, { share });
}

export function yearsText(tr: Translator, c: CareerCardView): string {
  // Tenths of a year: "4.2 years served" (Fluent's plural follows the number).
  const years = c.years_tenths / 10;
  return tr.t("ui-card-years", { years: Number.isInteger(years) ? years : Number(years.toFixed(1)) });
}

export function highestText(tr: Translator, c: CareerCardView): string {
  return c.highest_post ? postName(tr, c.highest_post) : tr.t("ui-card-highest-none");
}

export function endingTitle(tr: Translator, c: CareerCardView): string {
  return c.ending ? tr.t(c.ending.title_key) : "";
}

export function endingText(tr: Translator, c: CareerCardView): string {
  return c.ending ? tr.t(c.ending.text_key, actArgs(tr, c.ending.args)) : "";
}

/** When it ended, and as what. */
export function endedWhen(tr: Translator, c: CareerCardView): string {
  const date = formatDate(tr, c.ended_on);
  const post = c.ended_posts[0];
  return post ? tr.t("ui-card-ended-as", { date, post: postName(tr, post) }) : tr.t("ui-card-ended-out", { date });
}

export function figureText(tr: Translator, c: CareerCardView): string {
  const f = c.figure;
  if (f.rated === 0) return tr.t("ui-card-figure-none");
  return tr.t("ui-card-figure-value", { over: signed(f.over_pm), wins: f.wins, rated: f.rated, expected: (f.expected_pm / 1000).toFixed(2) });
}

/** A line's first letter in capitals (an arc's title opens lower case: "the plant"). */
const capital = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export function decisionLines(tr: Translator, c: CareerCardView): string[] {
  return c.decisions.map((d) => capital(tr.event(d.text_key, d.args)));
}

export function arcLines(tr: Translator, c: CareerCardView): string[] {
  return c.arcs.map((a) => {
    const title = capital(tr.event(a.title.key, a.title.args));
    return a.under_way ? `${title}: ${tr.t("ui-card-arc-under-way")}` : `${title}: ${tr.event(a.text.key, a.text.args)}`;
  });
}

/** A list cut to `max` lines, the rest counted. */
function cut(tr: Translator, lines: string[], max: number): string[] {
  if (lines.length <= max) return lines;
  return [...lines.slice(0, max), tr.t("ui-card-more", { n: lines.length - max })];
}

/** The short text block "Copy" gives (and "Share" sends beside the image). */
export function cardText(tr: Translator, c: CareerCardView): string {
  const out: string[] = [];
  out.push(tr.t(c.unfinished ? "ui-card-text-title-unfinished" : "ui-card-text-title"));
  out.push(whoText(tr, c));
  out.push(worldText(tr, c));
  out.push(`${tr.t("ui-card-opening")}: ${openingPlace(tr, c)}; ${ratingText(tr, c.opening.rating)}`);
  out.push(`${tr.t("ui-card-races")}:`);
  if (c.races.length === 0) out.push(`· ${tr.t("ui-card-races-none")}`);
  for (const r of c.races) out.push(`· ${raceHead(tr, r)}: ${ratingText(tr, r.rating)}; ${raceOutcome(tr, r)}`);
  out.push(`${tr.t("ui-card-highest")}: ${highestText(tr, c)} · ${yearsText(tr, c)}`);
  const decisions = decisionLines(tr, c);
  out.push(`${tr.t("ui-card-decisions")}: ${decisions.length ? cut(tr, decisions, TEXT_LIST_MAX).join(" · ") : tr.t("ui-card-decisions-none")}`);
  const arcs = arcLines(tr, c);
  out.push(`${tr.t("ui-card-arcs")}: ${arcs.length ? cut(tr, arcs, TEXT_LIST_MAX).join(" · ") : tr.t("ui-card-arcs-none")}`);
  out.push(`${tr.t("ui-card-ended")}: ${endingTitle(tr, c)}, ${endedWhen(tr, c)}`);
  out.push(`${tr.t("ui-card-figure")}: ${figureText(tr, c)}`);
  out.push(c.code ? `${tr.t("ui-card-code")}: ${c.code}` : tr.t("ui-card-code-none"));
  out.push(tr.t("ui-card-build", { build: c.build }));
  return out.join("\n");
}
