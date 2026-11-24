import { fireEvent, render, screen, within } from "@testing-library/react";
import { Calendar, ElectionNight, RaceCard } from "./CampaignCards";
import { postTitle, signerText } from "./posts";
import { useGame } from "../state/gameStore";
import { Translator } from "../i18n/fluent";
import { englishFtl } from "../i18n/englishLocale";
import type { CampaignView } from "../engine/protocol/CampaignView";
import type { NominationView } from "../engine/protocol/NominationView";
import type { PostDto } from "../engine/protocol/PostDto";

// A constitution whose local posts have other names (DEC-098): every label
// comes from the posts the engine sends, so the cards follow it.
const tr = new Translator("en", `${englishFtl()}\nbody-moot = Town moot\noffice-reeve = Reeve\nrole-alderman = Alderman\nrole-reeve = Reeve\n`);
const at = { kind: "municipality" as const, tier: "local", id: 1, name: "Ashford", name_key: "jurisdiction-municipality" };
const seat: PostDto = { kind: "seat", id: "moot", at, constituency_number: 3, label_key: "body-moot", role_key: "role-alderman" };
const office: PostDto = { kind: "office", id: "reeve", at, constituency_number: null, label_key: "office-reeve", role_key: "role-reeve" };
const nomination = (post: PostDto, over: Partial<NominationView> = {}): NominationView => ({
  post,
  held: false,
  open: true,
  reason_key: null,
  min_standing: null,
  chosen: false,
  holder: null,
  holder_party: null,
  holder_same_party: false,
  holder_outlook_key: null,
  gate_standing: null,
  standing: 55,
  warning_key: null,
  ...over,
});
const parties = [
  { id: 0, name: "Civic Union", short: "CU", colour: 0, stances: [] },
  { id: 1, name: "Green Left", short: "GL", colour: 1, stances: [] },
];

function campaign(over: Partial<CampaignView> = {}): CampaignView {
  return {
    election_day: null,
    campaign_opens: null,
    days_to_election: null,
    in_campaign: false,
    filing: null,
    nominations: [nomination(seat, { held: true, chosen: true }), nomination(office)],
    player_in_office: true,
    player_party: null,
    parties: [],
    races: [],
    results: [],
    last_election: null,
    seat_outcome: null,
    refusal: null,
    ...over,
  };
}

function setup() {
  const perform = vi.fn(async () => null);
  useGame.setState({ tr, busy: false, perform });
  return perform;
}

describe("filing and races follow the constitution's posts", () => {
  it("offers each post the engine lists, then standing down, and files for the post itself", () => {
    const perform = setup();
    render(<Calendar c={campaign()} />);
    const radios = within(screen.getByTestId("election-calendar")).getAllByRole("radio");
    expect(radios.map((r) => r.parentElement?.textContent)).toEqual(["Stand again: Ward 3", "Run for Reeve", "Stand down"]);
    expect((radios[0] as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByLabelText("Run for Reeve"));
    expect(perform).toHaveBeenCalledWith({ type: "file", post: office, retire: false });
    fireEvent.click(screen.getByLabelText("Stand down"));
    expect(perform).toHaveBeenLastCalledWith({ type: "file", post: null, retire: true });
  });

  it("closes a post the party will not back, and says why in the post's own words", () => {
    setup();
    render(<Calendar c={campaign({ nominations: [nomination(seat, { held: true, chosen: true }), nomination(office, { open: false, reason_key: "ui-nomination-standing", min_standing: 40 })] })} />);
    expect((screen.getByLabelText("Run for Reeve") as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByText("Your party will only back you for Reeve with Standing of 40 or more.")).toBeTruthy();
  });

  it("names the holder, the party's rule for its own holder and the gate before filing", () => {
    setup();
    const held = nomination(office, {
      holder: "Ann Kerr",
      holder_party: 0,
      holder_same_party: true,
      holder_outlook_key: "ui-holder-usually-stands",
      gate_standing: 40,
      standing: 55.4,
      warning_key: "ui-nomination-warn-seat",
    });
    render(<Calendar c={campaign({ parties, player_party: 0, nominations: [nomination(seat, { held: true, chosen: true }), held] })} />);
    const notes = screen.getByTestId("nomination-notes");
    expect(notes.textContent).toContain("Held by Ann Kerr");
    expect(notes.textContent).toContain("Civic Union");
    expect(notes.textContent).toContain("usually stands again");
    expect(notes.textContent).not.toMatch(/%/);
    expect(within(notes).getByText("Your party backs its own holder when they stand again.")).toBeTruthy();
    expect(within(notes).getByText("If Ann Kerr stands again, the party nominates them and you stand for your seat instead.")).toBeTruthy();
    expect(within(notes).getByText("Your party asks Standing of 40 for this. Yours is 55.")).toBeTruthy();
  });

  it("says an open post is open, and shows the party's refusal once the campaign opens", () => {
    setup();
    const refusal = {
      key: "nomination-refused",
      args: { why_key: "nomination-why-holder", then_key: "nomination-then-seat", holder: "Ann Kerr", post_key: "office-reeve", district_key: "ui-post-seat", tier: "local", n: "3", standing: "55", min: "40" },
    };
    render(<Calendar c={campaign({ in_campaign: true, refusal })} />);
    expect(screen.getByTestId("nomination-notes").textContent).toContain("Nobody holds it now.");
    expect(screen.getByTestId("nomination-refused").textContent).toBe("Your party has nominated Ann Kerr again for Reeve. You are standing for your seat in Ward 3.");
  });

  it("shows election night when the player was on no ballot: the races, who won, and their seat", () => {
    setup();
    const date = { day_number: 100, year: 2031, month: 11, day: 4 };
    const result = (post: PostDto, winner: string, party: number) => ({
      post,
      date,
      candidates: [
        { name: winner, party, votes: 900, share: 0.6, final_poll: null, is_player: false, is_winner: true, was_incumbent: false },
        { name: "Bo Lind", party: 1 - party, votes: 600, share: 0.4, final_poll: null, is_player: false, is_winner: false, was_incumbent: true },
      ],
      turnout: 0.5,
      decomposition: [],
      player_ran: false,
      player_won: false,
    });
    useGame.setState({
      electionNight: true,
      campaign: campaign({
        parties,
        last_election: date,
        results: [result(office, "Ann Kerr", 0), result(seat, "Cy Moss", 1)],
        seat_outcome: { post: seat, key: "ui-seat-passed", winner: "Cy Moss" },
      }),
    });
    render(<ElectionNight />);
    expect(screen.getByTestId("night-not-running").textContent).toBe("You were not on the ballot.");
    expect(screen.getByTestId("night-seat").textContent).toBe("Cy Moss now holds Ward 3, the post you held.");
    expect(screen.getByRole("heading", { name: "Reeve" })).toBeTruthy();
    expect(screen.getByTestId("night-seats").textContent).toContain("won by Cy Moss");
  });

  it("titles a race by its post", () => {
    setup();
    const race = { post: office, is_player_race: true, candidates: [], polls: [], poll_average: null, player_awareness: null };
    render(<RaceCard race={race} c={campaign()} />);
    expect(screen.getByRole("heading", { name: "Reeve" })).toBeTruthy();
    expect(postTitle(tr, seat)).toBe("Ward 3");
    expect(postTitle(tr, { ...seat, at: { ...at, tier: "national" } })).toBe("District 3");
  });

  it("names the signer by the office's label", () => {
    expect(signerText(tr, { office, lean: "lean_veto" })).toBe("Reeve: leaning to veto");
  });
});
