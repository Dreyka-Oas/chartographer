import { describe, expect, it } from "vitest";
import { PODIUM, podiumColor } from "./rank";

describe("podiumColor", () => {
  it("donne une couleur aux trois premiers rangs", () => {
    expect(podiumColor(0)).toBe(PODIUM[0]);
    expect(podiumColor(1)).toBe(PODIUM[1]);
    expect(podiumColor(2)).toBe(PODIUM[2]);
  });

  it("laisse les suivants sans couleur", () => {
    expect(podiumColor(3)).toBeNull();
    expect(podiumColor(42)).toBeNull();
  });

  it("garde trois teintes distinctes", () => {
    expect(new Set(PODIUM).size).toBe(3);
  });
});
