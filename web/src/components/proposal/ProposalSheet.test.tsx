/// <reference types="node" />
// The proposal sheet (DEC-092, F6) against the engine's views
// (fixtures/proposal/*.json, written by crates/mandate-wasm/tests/proposal_sheet.rs):
// the desk lists the office's proposals and opens one; every option shows the
// cost, support and estimate the preview gives for it; a choice asks the
// engine again and shows its answer; a sponsor's version is its terms and its
// price; drafting and amending send the engine's own reading of the package;
// below "known" the sheet is a stub with its lead. Names, ids and counts come
// from the fixtures, never from the test.
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { readFileSync, existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { BillView } from "../../engine/protocol/BillView";
import type { ChoiceValue } from "../../engine/protocol/ChoiceValue";
import type { OfficeView } from "../../engine/protocol/OfficeView";
import type { PolicyPreviewView } from "../../engine/protocol/PolicyPreviewView";
import type { ProposalView } from "../../engine/protocol/ProposalView";
import type { QueryDto } from "../../engine/protocol/QueryDto";
import { englishTranslator } from "../../i18n/englishLocale";
import { formatMoney } from "../../i18n/format";
import { useGame } from "../../state/gameStore";
import { BillCard, impactLine } from "../BillCard";
import { DeskProposals, ProposalsHere } from "./ProposalList";
import { ProposalSheet, priceText, proposalTitle } from "./ProposalSheet";
import { useProposal } from "./proposalStore";

const request = vi.fn();
vi.mock("../../engine/client", async (original) => ({
  ...(await original<typeof import("../../engine/client")>()),
  engine: () => ({ request }),
}));

const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), "../fixtures/proposal");
const load = <T,>(name: string) => JSON.parse(readFileSync(path.join(dir, `${name}.json`), "utf8")) as T;
const has = (name: string) => existsSync(path.join(dir, `${name}.json`));

const tr = englishTranslator();
const proposals = load<ProposalView[]>("proposals");
const known = proposals.find((p) => p.level === "known") as ProposalView;
const preview = load<PolicyPreviewView>("preview");
const swapped = load<PolicyPreviewView>("preview-option");
const adopted = load<PolicyPreviewView>("preview-sponsor");
const bill = load<BillView>("bill");

const sameChoices = (a: ChoiceValue[], b: ChoiceValue[]) =>
  a.length === b.length && a.every((c) => b.some((d) => d.axis === c.axis && d.option === c.option));

/** The axis and option the "option" fixture chose that the default did not. */
function swappedChoice(): ChoiceValue {
  const d = preview.family?.choices ?? [];
  const c = (swapped.family?.choices ?? []).find((x) => !d.some((y) => y.axis === x.axis && y.option === x.option));
  if (!c) throw new Error("the option fixture chose nothing new");
  return c;
}

/** The engine: each fixture answers the package it was made for. */
function engineAnswers(q: QueryDto) {
  if (q.type !== "policy_preview") return Promise.reject(new Error(q.type));
  const choices = q.choices ?? [];
  const heard = proposals.find((p) => p.level === "heard");
  if (heard && q.proposal === heard.id && has("preview-heard")) return Promise.resolve({ kind: "policy_preview", payload: load<PolicyPreviewView>("preview-heard") });
  if (q.proposal !== known.id || q.template !== known.family) return Promise.reject(new Error("no such proposal"));
  const sponsor = preview.family?.sponsors[0];
  let view = preview;
  if (choices.some((c) => c.axis === swappedChoice().axis && c.option === swappedChoice().option)) view = swapped;
  else if (sponsor && choices.length > 0 && sameChoices(choices, sponsor.preset)) view = adopted;
  return Promise.resolve({ kind: "policy_preview", payload: view });
}

function setup(office: Partial<OfficeView> = {}) {
  const perform = vi.fn(() => Promise.resolve(bill.id as number | null));
  request.mockReset();
  request.mockImplementation((m: { payload: QueryDto }) => engineAnswers(m.payload));
  useProposal.setState({ open: null });
  useGame.setState({
    tr,
    busy: false,
    error: null,
    perform,
    visit: vi.fn(() => Promise.resolve()),
    office: { proposals, bills: [bill], ...office } as unknown as OfficeView,
    folds: {},
  });
  return { perform };
}

async function openKnown() {
  render(
    <>
      <DeskProposals />
      <ProposalSheet />
    </>,
  );
  fireEvent.click(screen.getByTestId(`open-proposal-${known.id}`));
  const sheet = await screen.findByTestId("proposal-sheet");
  await waitFor(() => expect(within(sheet).getByTestId("proposal-package")).toBeInTheDocument());
  return sheet;
}

const noRawText = (el: HTMLElement) => {
  expect(el.textContent).not.toContain("⟦");
  expect(el.textContent).not.toMatch(/\{\s*\$/);
};

describe("the proposal sheet", () => {
  it("is listed on the desk as the office gives it, and a row opens the engine's preview", async () => {
    setup();
    const sheet = await openKnown();
    const list = screen.getByTestId("desk-proposal-list");
    expect(within(list).getAllByRole("listitem")).toHaveLength(proposals.length);
    expect(within(list).getByTestId(`proposal-row-${known.id}`)).toHaveTextContent(formatMoney(known.est_annual_cost ?? 0));
    expect(request).toHaveBeenCalledWith({ kind: "query", payload: { type: "policy_preview", template: known.family, params: [], proposal: known.id, choices: [] } });
    expect(within(sheet).getByRole("heading", { level: 2 })).toHaveTextContent(proposalTitle(tr, known));
    noRawText(sheet);
  });

  it("shows every option's cost, support and estimate as the preview gives them, the chosen one checked", async () => {
    setup();
    const sheet = await openKnown();
    const fam = preview.family;
    if (!fam) throw new Error("no family part");
    expect(within(sheet).getByTestId("proposal-cost")).toHaveTextContent(formatMoney(preview.est_annual_cost));
    for (const a of fam.axes) {
      const axis = within(sheet).getByTestId(`proposal-axis-${a.id}`);
      expect(axis.tagName).toBe("FIELDSET");
      for (const o of a.options) {
        const row = within(axis).getByTestId(`proposal-option-${a.id}-${o.id}`);
        if (o.available) expect(within(row).getByTestId("option-cost")).toHaveTextContent(formatMoney(o.est_annual_cost));
        else expect(within(row).queryByTestId("option-cost")).toBeNull();
        expect(within(row).getByRole("radio")).toHaveProperty("checked", o.chosen);
        if (o.support) expect(row).toHaveTextContent(tr.t(`support-band-${o.support.band}`));
        // The estimate's own line, with the engine's range: never a point it did not give.
        const lead = o.impact?.lines.find((l) => l.lead);
        if (lead) expect(within(row).getByTestId("option-impact")).toHaveTextContent(impactLine(tr, lead));
      }
    }
    noRawText(sheet);
  });

  it("asks the engine again when an option is chosen, and shows its answer", async () => {
    setup();
    const sheet = await openKnown();
    const c = swappedChoice();
    const row = within(sheet).getByTestId(`proposal-option-${c.axis}-${c.option}`);
    await act(async () => {
      fireEvent.click(within(row).getByRole("radio"));
    });
    const last = request.mock.calls.at(-1)?.[0] as { payload: Extract<QueryDto, { type: "policy_preview" }> };
    expect(last.payload.choices).toContainEqual(c);
    await waitFor(() => expect(within(sheet).getByTestId("proposal-cost")).toHaveTextContent(formatMoney(swapped.est_annual_cost)));
    expect(within(sheet).getByTestId(`proposal-option-${c.axis}-${c.option}`)).toHaveAttribute("data-chosen", "true");
  });

  it("shows the UI's figures only from the engine: an arbitrary answer is shown as given", async () => {
    setup();
    const odd = structuredClone(preview);
    odd.est_annual_cost = 31_337;
    const first = odd.family?.axes[0]?.options[0];
    if (first) first.est_annual_cost = 27_183;
    request.mockImplementation(() => Promise.resolve({ kind: "policy_preview", payload: odd }));
    const sheet = await openKnown();
    expect(within(sheet).getByTestId("proposal-cost")).toHaveTextContent(formatMoney(31_337));
    if (first) expect(sheet).toHaveTextContent(formatMoney(27_183));
  });

  it("lists the sponsors with their terms and price, and takes a version as the engine reads it", async () => {
    setup();
    const sheet = await openKnown();
    const fam = preview.family;
    const sponsor = fam?.sponsors[0];
    if (!fam || !sponsor) throw new Error("no sponsor");
    for (const s of fam.sponsors) expect(within(within(sheet).getByTestId(`proposal-sponsor-${s.id}`)).getByTestId("sponsor-price")).toHaveTextContent(priceText(tr, s.price));
    await act(async () => {
      fireEvent.click(within(within(sheet).getByTestId(`proposal-sponsor-${sponsor.id}`)).getByTestId("sponsor-take"));
    });
    const last = request.mock.calls.at(-1)?.[0] as { payload: Extract<QueryDto, { type: "policy_preview" }> };
    expect(sameChoices(last.payload.choices ?? [], sponsor.preset)).toBe(true);
    expect(last.payload.params).toEqual(sponsor.params);
    const taken = adopted.family?.sponsors.find((s) => s.matches_current);
    if (!taken) throw new Error("no sponsor matches");
    await waitFor(() => expect(within(sheet).getByTestId(`proposal-sponsor-${taken.id}`)).toHaveAttribute("data-matches", "true"));
    expect(within(sheet).getByTestId("proposal-draft")).toHaveTextContent(tr.t("ui-proposal-draft-sponsor"));
  });

  it("drafts with the engine's reading of the package, the sponsor only when their version is as it is, and says it is done", async () => {
    const { perform } = setup();
    const sheet = await openKnown();
    const fam = preview.family;
    if (!fam) throw new Error("no family part");
    await act(async () => {
      fireEvent.click(within(sheet).getByTestId("proposal-draft"));
    });
    const sponsor = fam.sponsors.find((s) => s.matches_current)?.id ?? null;
    expect(perform).toHaveBeenCalledWith({ type: "draft_family", proposal: known.id, choices: fam.choices, params: preview.params, sponsor });
    expect(await within(sheet).findByTestId("proposal-result")).toHaveAttribute("data-kind", "drafted");
  });

  it("amends a bill drafted from it: its choices in, the bill's clauses kept", async () => {
    const { perform } = setup();
    const sheet = await openKnown();
    expect(known.bills).toContain(bill.id);
    const listed = within(sheet).getByTestId(`proposal-bill-${bill.id}`);
    await act(async () => {
      fireEvent.click(within(listed).getByTestId("proposal-bill-amend"));
    });
    expect(await within(sheet).findByTestId("proposal-amending")).toBeInTheDocument();
    const last = request.mock.calls.at(-1)?.[0] as { payload: Extract<QueryDto, { type: "policy_preview" }> };
    expect(sameChoices(last.payload.choices ?? [], bill.family?.choices ?? [])).toBe(true);
    await waitFor(() => expect(within(sheet).getByTestId("proposal-amend")).toBeEnabled());
    await act(async () => {
      fireEvent.click(within(sheet).getByTestId("proposal-amend"));
    });
    const view = useProposal.getState().open?.view;
    expect(perform).toHaveBeenCalledWith({ type: "amend_policy", policy: bill.id, params: view?.params, clauses: bill.clauses, choices: view?.family?.choices });
    expect(await within(sheet).findByTestId("proposal-result")).toHaveAttribute("data-kind", "amended");
  });

  it("below 'known' is a stub: the reason and the lead, no menu and no draft", async () => {
    const heard = proposals.find((p) => p.level === "heard");
    if (!heard || !has("preview-heard")) return; // the world had one proposal open
    setup();
    render(
      <>
        <ProposalsHere proposals={[heard]} />
        <ProposalSheet />
      </>,
    );
    fireEvent.click(screen.getByTestId(`open-proposal-${heard.id}`));
    const stub = await screen.findByTestId("proposal-stub");
    const sheet = screen.getByTestId("proposal-sheet");
    expect(sheet).toHaveAttribute("data-level", "heard");
    if (heard.reason_key) expect(stub).toHaveTextContent(tr.event(heard.reason_key, heard.args));
    if (heard.knowledge.lead) expect(within(sheet).getByTestId("notebook-lead")).toHaveTextContent(tr.event(heard.knowledge.lead.key, heard.knowledge.lead.args));
    expect(within(sheet).queryByTestId("proposal-draft")).toBeNull();
    expect(within(sheet).queryAllByRole("radio")).toHaveLength(0);
    noRawText(sheet);
  });

  it("names a family bill with its proposal's arguments and amends it on the sheet", async () => {
    setup();
    render(
      <>
        <BillCard bill={bill} template={undefined} town={{ wards: [], neighborhoods: [] } as unknown as OfficeView["town"]} />
        <ProposalSheet />
      </>,
    );
    const card = screen.getByTestId(`bill-${bill.id}`);
    noRawText(card);
    expect(within(card).getByRole("heading", { level: 3 }).textContent?.toLowerCase()).toBe(tr.event(bill.name_key, bill.family?.args ?? {}).toLowerCase());
    await act(async () => {
      fireEvent.click(within(card).getByTestId("bill-amend-proposal"));
    });
    expect(await screen.findByTestId("proposal-amending")).toBeInTheDocument();
  });
});
