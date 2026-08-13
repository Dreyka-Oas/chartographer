import { describe, expect, it } from "vitest";
import { anchorX, placeBubble } from "./tooltipPlacement";

describe("placement de la bulle", () => {
  it("se pose au-dessus quand la place y est", () => {
    const placed = placeBubble({ top: 400, bottom: 420 }, 90, 900);
    expect(placed.below).toBe(false);
    expect(placed.top).toBe(392);
  });

  /**
   * Le cas qui rognait les explications : une bulle haute, ancrée près du haut
   * de la fenêtre. Supposer sa hauteur la laissait déborder.
   */
  it("bascule en dessous quand elle deborderait par le haut", () => {
    const placed = placeBubble({ top: 60, bottom: 78 }, 90, 900);
    expect(placed.below).toBe(true);
    expect(placed.top).toBe(86);
  });

  it("revient au-dessus quand le bas manque", () => {
    const placed = placeBubble({ top: 800, bottom: 820 }, 90, 900, "bottom");
    expect(placed.below).toBe(false);
    expect(placed.top).toBe(792);
  });

  /** Plus haute que la place des deux côtés : elle se colle au bord haut. */
  it("se colle en haut quand elle ne tient nulle part", () => {
    const placed = placeBubble({ top: 200, bottom: 220 }, 700, 400);
    expect(placed.below).toBe(false);
    expect(placed.top).toBe(710);
  });
});

describe("ancrage horizontal", () => {
  it("laisse la bulle centree quand rien ne la gene", () => {
    expect(anchorX(600, 260, 1400)).toBe(600);
  });

  it("ecarte du bord droit de la moitie de la bulle", () => {
    // 1400 - 10 - 130 : au-delà, la bulle sortirait de la vue.
    expect(anchorX(1390, 260, 1400)).toBe(1260);
  });

  it("ecarte du bord gauche de la meme facon", () => {
    expect(anchorX(4, 260, 1400)).toBe(140);
  });

  /** Fenêtre plus étroite que la bulle : elle se centre, faute de mieux. */
  it("centre quand la vue est plus etroite que la bulle", () => {
    expect(anchorX(10, 260, 200)).toBe(100);
  });
});
