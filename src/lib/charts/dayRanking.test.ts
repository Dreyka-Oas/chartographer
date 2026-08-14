import { describe, expect, it } from "vitest";
import { PODIUM } from "../components/rank";
import type { DayRankRow } from "../types";
import { dailyBarsOption, rankCurveOption } from "./dayRanking";

function row(partial: Partial<DayRankRow>): DayRankRow {
  return {
    day: "2026-08-10",
    modrinth: 0,
    curseforge: 0,
    total: 0,
    revenue: "0",
    rank_period: null,
    rank_at_the_time: null,
    compared_days: 0,
    ...partial,
  };
}

const rows = [
  row({ day: "2026-08-09", modrinth: 40, curseforge: 10, total: 50, rank_period: 2, rank_at_the_time: 1 }),
  row({ day: "2026-08-10", modrinth: 80, curseforge: 20, total: 100, rank_period: 1, rank_at_the_time: 1 }),
];

describe("dailyBarsOption", () => {
  it("porte une série par plateforme, dans l'ordre des jours", () => {
    const option = dailyBarsOption(rows);
    const modrinth = option.series.find((s) => s.id === "day:modrinth");
    expect(modrinth?.data.map((d) => d.value)).toEqual([40, 80]);
    expect(option.xAxis.data).toEqual(["2026-08-09", "2026-08-10"]);
  });

  it("marque la meilleure journée de la période à la couleur du podium", () => {
    const option = dailyBarsOption(rows);
    const curseforge = option.series.find((s) => s.id === "day:curseforge");
    // Le liseré du podium coiffe la pile, donc la série du haut.
    expect(curseforge?.data[1].itemStyle?.borderColor).toBe(PODIUM[0]);
    expect(curseforge?.data[0].itemStyle?.borderColor).toBe(PODIUM[1]);
  });
});

describe("rankCurveOption", () => {
  it("met le premier rang en haut", () => {
    const option = rankCurveOption(rows);
    expect(option.yAxis.inverse).toBe(true);
    expect(option.yAxis.min).toBe(1);
  });

  it("laisse un trou pour les journées sans rang", () => {
    const option = rankCurveOption([...rows, row({ day: "2026-08-11" })]);
    expect(option.series[0].data[2]).toBeNull();
  });

  it("formate le rang en entier brut, pas en nombre abrégé", () => {
    const option = rankCurveOption(rows);
    expect(option.tooltip.formatter).toBeInstanceOf(Function);
    // `compactNumber` écrirait "1" pareil ici : le test vaut pour les rangs à
    // trois chiffres et plus, où l'abréviation diverge du nombre brut.
    const html = option.tooltip.formatter([
      { axisValue: "2026-08-10", marker: "", seriesName: "Rang du jour", value: 1200, data: 1200 },
    ]);
    expect(html).toContain("1200");
  });
});
