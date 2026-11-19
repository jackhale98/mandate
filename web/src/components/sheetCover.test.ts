import { SHEET_COVER, coverShare } from "./sheetCover";

describe("how much of the street a sheet covers", () => {
  const street = { left: 0, top: 0, right: 400, bottom: 800 };
  it("a phone's bottom sheet: the share of the street's height it takes", () => {
    expect(coverShare(street, { left: 0, top: 240, right: 400, bottom: 900 })).toBeCloseTo(0.7);
    expect(coverShare(street, { left: 0, top: 560, right: 400, bottom: 900 })).toBeCloseTo(0.3);
    expect(coverShare(street, { left: 0, top: 240, right: 400, bottom: 900 }) > SHEET_COVER).toBe(true);
  });
  it("beside the street (docked) or nowhere: none", () => {
    expect(coverShare(street, { left: 400, top: 0, right: 800, bottom: 800 })).toBe(0);
    expect(coverShare({ left: 0, top: 0, right: 0, bottom: 0 }, street)).toBe(0);
  });
});
