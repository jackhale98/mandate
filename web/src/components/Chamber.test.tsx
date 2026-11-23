import { render, screen } from "@testing-library/react";
import { YourVote } from "./Chamber";
import { useGame } from "../state/gameStore";
import { Translator } from "../i18n/fluent";
import type { BillView } from "../engine/protocol/BillView";

const FTL = `
ui-your-vote = Your vote
ui-party-line-yes = Your party expects you to vote yes.
ui-party-line-no = Your party expects you to vote no.
ui-party-line-free = A free vote: your party has no line.
ui-ward-view = Your ward's view: { $lean }. { $for }% for, { $against }% against ({ $certainty }).
ward-lean-leans_for = leans for
ward-lean-strongly_against = strongly against
ward-certainty-rough = rough estimate
ward-certainty-exact = exact
party-reason-fits = It fits their stance on { $issue }.
party-reason-free-torn = Their stance on { $issue } and who proposed it pull in opposite directions.
party-issue-housing = housing
ui-vote-yes = Vote yes
ui-vote-no = Vote no
ui-vote-abstain = Abstain
ui-vote-help = Public and final.
`;

function bill(over: Partial<BillView>): BillView {
  return {
    id: 1,
    can_vote: true,
    my_vote: null,
    owe_sponsor: false,
    sponsor_name: "",
    party_line: "yes",
    party_line_reason: { key: "party-reason-fits", args: { issue: "party-issue-housing" } },
    ward_support: 0.2,
    ward_lean: "leans_for",
    ward_for_pct: 60,
    ward_against_pct: 40,
    ward_certainty: "rough",
    ...over,
  } as unknown as BillView;
}

describe("YourVote", () => {
  beforeEach(() => {
    useGame.setState({ tr: new Translator("en", FTL), busy: false });
  });

  it("states the party line with its reason, in words", () => {
    render(<YourVote bill={bill({})} />);
    expect(screen.getByTestId("party-line-reason")).toHaveTextContent("It fits their stance on housing.");
    expect(screen.getByTestId("your-vote")).toHaveTextContent("Your party expects you to vote yes.");
  });

  it("shows the ward as a lean word, shares and how sure the estimate is, never a net number", () => {
    render(<YourVote bill={bill({})} />);
    const ward = screen.getByTestId("ward-view");
    expect(ward).toHaveTextContent("Your ward's view: leans for. 60% for, 40% against (rough estimate).");
    expect(ward.textContent).not.toMatch(/\+20/);
  });

  it("shows exact figures at the highest knowledge, and a free vote's reason", () => {
    render(
      <YourVote
        bill={bill({
          party_line: "free",
          party_line_reason: { key: "party-reason-free-torn", args: { issue: "party-issue-housing" } },
          ward_lean: "strongly_against",
          ward_for_pct: 7,
          ward_against_pct: 93,
          ward_certainty: "exact",
        })}
      />,
    );
    expect(screen.getByTestId("ward-view")).toHaveTextContent("strongly against. 7% for, 93% against (exact).");
    expect(screen.getByTestId("party-line-reason")).toHaveTextContent("Their stance on housing and who proposed it pull in opposite directions.");
  });
});
