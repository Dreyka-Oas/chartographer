import { describe, expect, it } from "vitest";
import { addDays, monthGrid, shiftMonth } from "./calendar";

describe("monthGrid", () => {
  it("rend six semaines de sept jours, quel que soit le mois", () => {
    for (const month of ["2026-02", "2026-08", "2027-01"]) {
      const grid = monthGrid(month);
      expect(grid).toHaveLength(6);
      expect(grid.every((week) => week.length === 7)).toBe(true);
    }
  });

  it("commence la semaine au lundi", () => {
    // Le 1er août 2026 est un samedi : la première semaine s'ouvre le lundi 27 juillet.
    expect(monthGrid("2026-08")[0][0]).toBe("2026-07-27");
  });

  it("couvre le mois entier sans trou ni doublon", () => {
    const days = monthGrid("2026-02").flat();
    expect(new Set(days).size).toBe(days.length);
    expect(days).toContain("2026-02-01");
    expect(days).toContain("2026-02-28");
  });

  /** Février 2028 est bissextile : le 29 doit être là, et une seule fois. */
  it("connaît les années bissextiles", () => {
    const days = monthGrid("2028-02").flat();
    expect(days.filter((day) => day === "2028-02-29")).toHaveLength(1);
  });
});

describe("shiftMonth", () => {
  it("passe d'une année à l'autre", () => {
    expect(shiftMonth("2026-01", -1)).toBe("2025-12");
    expect(shiftMonth("2026-12", 1)).toBe("2027-01");
  });
});

describe("addDays", () => {
  it("passe d'un mois à l'autre", () => {
    expect(addDays("2026-08-30", 3)).toBe("2026-09-02");
  });

  it("passe d'une année à l'autre", () => {
    expect(addDays("2026-12-30", 3)).toBe("2027-01-02");
  });

  /** 2028 est bissextile : un pas qui traverse le 29 février doit le compter. */
  it("traverse le 29 février d'une année bissextile", () => {
    expect(addDays("2028-02-27", 3)).toBe("2028-03-01");
  });
});
