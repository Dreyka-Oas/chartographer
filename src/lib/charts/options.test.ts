import { describe, expect, it } from "vitest";
import type { ProjectSummary } from "../types";
import { heatmapOption } from "./heatmap";
import { revenueOption } from "./revenue";
import { splitOption } from "./split";
import { timelineOption } from "./timeline";
import { worldMapOption } from "./worldmap";

const points = [
  { day: "2026-08-09", modrinth: 40, curseforge: 0 },
  { day: "2026-08-10", modrinth: 55, curseforge: 75 },
];

function project(partial: Partial<ProjectSummary>): ProjectSummary {
  return {
    key: "k",
    title: "T",
    icon_url: null,
    modrinth_id: 1,
    curseforge_id: 2,
    modrinth_downloads: 0,
    curseforge_downloads: 0,
    followers: 0,
    link_confidence: 1,
    spark: [],
    ...partial,
  };
}

describe("timelineOption", () => {
  it("produit deux séries empilées alignées sur les jours", () => {
    const option = timelineOption(points, true);
    expect(option.xAxis.data).toEqual(["2026-08-09", "2026-08-10"]);
    expect(option.series).toHaveLength(2);
    expect(option.series[0].data).toEqual([40, 55]);
    expect(option.series[1].data).toEqual([0, 75]);
    expect(option.series[0].stack).toBe(option.series[1].stack);
  });

  it("désempile quand le mode comparaison est actif", () => {
    const option = timelineOption(points, false);
    expect(option.series[0].stack).toBeUndefined();
  });
});

describe("splitOption", () => {
  it("trie par volume décroissant, le plus gros en haut de l'axe", () => {
    const option = splitOption([
      project({ key: "a", title: "Petit", modrinth_downloads: 10, curseforge_downloads: 12 }),
      project({ key: "b", title: "Gros", modrinth_downloads: 23225, curseforge_downloads: 86753 }),
    ]);
    expect(option.yAxis.data[option.yAxis.data.length - 1]).toBe("Gros");
  });
});

describe("heatmapOption", () => {
  it("indexe les cellules sur les axes des versions et des loaders", () => {
    const option = heatmapOption([
      { game_version: "1.21", loader: "fabric", downloads: 40 },
      { game_version: "1.20.1", loader: "neoforge", downloads: 10 },
    ]);
    expect(option.xAxis.data).toContain("1.21");
    expect(option.yAxis.data).toContain("neoforge");
    expect(option.series[0].data).toHaveLength(2);
    expect(option.visualMap.max).toBe(40);
  });

  it("ordonne les versions de jeu numériquement", () => {
    const option = heatmapOption([
      { game_version: "1.21", loader: "fabric", downloads: 1 },
      { game_version: "1.9", loader: "fabric", downloads: 1 },
      { game_version: "1.20.1", loader: "fabric", downloads: 1 },
    ]);
    expect(option.xAxis.data).toEqual(["1.9", "1.20.1", "1.21"]);
  });
});

describe("worldMapOption", () => {
  it("exclut le pays inconnu de la carte", () => {
    const option = worldMapOption([
      { country: "DE", downloads: 88 },
      { country: "??", downloads: 1012 },
    ]);
    expect(option.series[0].data.map((d) => d.name)).toEqual(["DE"]);
  });
});

describe("revenueOption", () => {
  it("cumule les montants journaliers", () => {
    const option = revenueOption([
      { day: "2026-08-09", amount: "0.5" },
      { day: "2026-08-10", amount: "0.25" },
    ]);
    expect(option.series[0].data).toEqual([0.5, 0.25]);
    expect(option.series[1].data).toEqual([0.5, 0.75]);
  });
});
