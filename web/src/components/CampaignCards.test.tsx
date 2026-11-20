import { fireEvent, render, screen, within } from "@testing-library/react";
import { Calendar, RaceCard } from "./CampaignCards";
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
const nomination = (post: PostDto, over: Partial<NominationView> = {}): NominationView => ({ post, held: false, open: true, reason_key: null, min_standing: null, chosen: false, ...over });

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
