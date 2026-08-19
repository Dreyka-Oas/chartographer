import { describe, expect, it } from "vitest";
import type { ProjectSummary } from "../types";
import { cellTooltipHtml, heatmapOption } from "./heatmap";
import { foldSeriesTail, stackedProjectsOption, stackValues } from "./multiseries";
import { revenueOption } from "./revenue";
import { sparklinePath } from "./sparkline";
import { splitOption } from "./split";
import { DARK, dayAxis, dayTooltipHtml, escapeHtml, monthAxis } from "./theme";
import { timelineOption } from "./timeline";
import { countryTooltipHtml, fillZoom, MAP_ASPECT, worldMapOption } from "./worldmap";

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
    modrinth_ext_id: "AABBCCDD",
    curseforge_ext_id: 1002185,
    modrinth_downloads: 0,
    curseforge_downloads: 0,
    followers: 0,
    link_confidence: 1,
    spark: [],
    ...partial,
  };
}

/** Une série d'option, telle que les fabriques la rendent. */
function line(option: { series: { id: string; data: { value: number; own: number }[] }[] }, id: string) {
  const found = option.series.find((s) => s.id === id);
  if (!found) throw new Error(`série absente : ${id}`);
  return found;
}

describe("timelineOption", () => {
  it("empile les plateformes en gardant la valeur propre de chacune", () => {
    const option = timelineOption(points, true);
    expect(option.xAxis.data).toEqual(["2026-08-09", "2026-08-10"]);
    expect(option.series).toHaveLength(2);
    expect(line(option, "platform:modrinth").data).toEqual([
      { value: 40, own: 40 },
      { value: 55, own: 55 },
    ]);
    // CurseForge est dessinée au niveau du total ; son chiffre à elle reste
    // joint, c'est lui qu'annonce le tooltip.
    expect(line(option, "platform:curseforge").data).toEqual([
      { value: 40, own: 0 },
      { value: 130, own: 75 },
    ]);
  });

  it("peint la plus haute en premier pour ne pas couvrir l'autre", () => {
    const option = timelineOption(points, true);
    expect(option.series[0].id).toBe("platform:curseforge");
    expect(option.series[1].id).toBe("platform:modrinth");
  });

  it("rend les valeurs brutes quand le mode comparaison est actif", () => {
    const option = timelineOption(points, false);
    expect(option.series[0].id).toBe("platform:modrinth");
    expect(line(option, "platform:curseforge").data).toEqual([
      { value: 0, own: 0 },
      { value: 75, own: 75 },
    ]);
  });
});

describe("stackValues", () => {
  it("cumule les séries dans l'ordre, jour par jour", () => {
    expect(stackValues([[1, 2], [10, 20], [100, 200]])).toEqual([
      [1, 2],
      [11, 22],
      [111, 222],
    ]);
  });
});

describe("stackedProjectsOption", () => {
  const days = ["2026-08-09", "2026-08-10"];
  const series = [
    { name: "Gros", values: [100, 120] },
    { name: "Petit", values: [5, 8] },
  ];

  it("empile les mods et garde leur chiffre propre", () => {
    const option = stackedProjectsOption(days, series, DARK, true);
    expect(line(option, "mod:Petit").data).toEqual([
      { value: 105, own: 5 },
      { value: 128, own: 8 },
    ]);
    // Le plus haut cumul d'abord, sinon son aire recouvrirait les autres.
    expect(option.series[0].id).toBe("mod:Petit");
  });

  it("laisse les courbes à leur hauteur propre une fois désempilées", () => {
    const option = stackedProjectsOption(days, series, DARK, false);
    expect(line(option, "mod:Petit").data).toEqual([
      { value: 5, own: 5 },
      { value: 8, own: 8 },
    ]);
    expect(option.series[0].id).toBe("mod:Gros");
  });
});

describe("foldSeriesTail", () => {
  const series = [
    { name: "a", values: [5, 5] },
    { name: "b", values: [3, 0] },
    { name: "c", values: [1, 2] },
    { name: "d", values: [1, 1] },
  ];

  it("laisse la liste intacte sous le plafond", () => {
    expect(foldSeriesTail(series, 4)).toBe(series);
  });

  it("somme la queue jour par jour sans rien perdre", () => {
    const folded = foldSeriesTail(series, 2);
    expect(folded).toHaveLength(3);
    expect(folded[2].name).toBe("2 autres mods");
    expect(folded[2].values).toEqual([2, 3]);
  });
});

describe("sparklinePath", () => {
  it("normalise sur le maximum et referme l'aire sur la ligne de base", () => {
    const { line, area } = sparklinePath([0, 5, 10], 100, 30);
    expect(line.startsWith("M0,30C")).toBe(true);
    expect(line).toContain("100,0");
    expect(area).toBe(`${line}L100,30L0,30Z`);
  });

  it("aplatit une série entièrement nulle au lieu de diviser par zéro", () => {
    const { line } = sparklinePath([0, 0, 0], 100, 30);
    expect(line).toBe("M0,30C8.33,30 33.33,30 50,30C66.67,30 91.67,30 100,30");
  });

  it("garde la courbe dans la boîte malgré un pic isolé", () => {
    const { line } = sparklinePath([0, 0, 10, 0, 0], 100, 30);
    const ys = [...line.matchAll(/-?[\d.]+,(-?[\d.]+)/g)].map((m) => Number(m[1]));
    expect(Math.min(...ys)).toBeGreaterThanOrEqual(0);
    expect(Math.max(...ys)).toBeLessThanOrEqual(30);
  });

  it("ne trace rien sous deux points", () => {
    expect(sparklinePath([7]).line).toBe("");
    expect(sparklinePath([]).area).toBe("");
  });
});

describe("dayAxis", () => {
  it("garde les dates ISO en données et n'affiche que jour et mois", () => {
    const axis = dayAxis(["2026-08-09", "2026-08-10"], DARK);
    expect(axis.data).toEqual(["2026-08-09", "2026-08-10"]);
    expect(axis.axisLabel.formatter("2026-08-09")).toBe("9 août");
  });
});

describe("monthAxis", () => {
  it("écrit les mois en toutes lettres", () => {
    const axis = monthAxis(["2026-08"], DARK);
    expect(axis.axisLabel.formatter("2026-08")).toBe("août 2026");
  });
});

describe("dayTooltipHtml", () => {
  it("titre la bulle avec la date complète", () => {
    const html = dayTooltipHtml([
      { axisValue: "2026-08-09", marker: "●", seriesName: "Modrinth", value: 1776 },
    ]);
    expect(html).toContain("9 août 2026");
    expect(html).toContain("Modrinth");
    expect(html.replace(/\s/g, " ")).toContain("1,8 k");
    expect(html).not.toContain("Total");
  });

  it("ajoute le total quand plusieurs séries sont survolées", () => {
    const html = dayTooltipHtml([
      { axisValue: "2026-08-09", marker: "●", seriesName: "Modrinth", value: 1000 },
      { axisValue: "2026-08-09", marker: "●", seriesName: "CurseForge", value: 240 },
    ]);
    expect(html).toContain("Total");
    expect(html.replace(/\s/g, " ")).toContain("1,2 k");
  });

  /**
   * L'empilement étant calculé par l'application, la valeur portée par le point
   * est une hauteur cumulée. Annoncer celle-ci gonflerait chaque mod et
   * doublerait le total.
   */
  it("annonce la valeur propre plutot que le cumul dessine", () => {
    const html = dayTooltipHtml([
      { axisValue: "2026-08-09", seriesName: "Modrinth", value: 1000, data: { value: 1000, own: 1000 } },
      { axisValue: "2026-08-09", seriesName: "CurseForge", value: 1240, data: { value: 1240, own: 240 } },
    ]);
    const flat = html.replace(/\s/g, " ");
    expect(flat).toContain("240");
    expect(flat).not.toContain("1,2 k</b><br>");
    expect(flat).toContain("Total <b>1,2 k");
  });

  it("classe les series de la plus grosse a la plus petite quand on le demande", () => {
    const html = dayTooltipHtml(
      [
        { axisValue: "2026-08-09", seriesName: "Petit", value: 10, data: { value: 1000, own: 10 } },
        { axisValue: "2026-08-09", seriesName: "Gros", value: 990, data: { value: 990, own: 990 } },
      ],
      { sorted: true },
    );
    expect(html.indexOf("Gros")).toBeLessThan(html.indexOf("Petit"));
  });

  /** Le logo est ce qui distingue douze courbes plus vite qu'une pastille. */
  it("pose le logo du mod devant son nom", () => {
    const html = dayTooltipHtml(
      [{ axisValue: "2026-08-09", seriesName: "Vein Vantage", value: 887 }],
      { icon: () => "https://cdn.modrinth.com/data/abc/icon.png" },
    );
    expect(html).toContain('<img src="https://cdn.modrinth.com/data/abc/icon.png"');
    expect(html.indexOf("icon.png")).toBeLessThan(html.indexOf("Vein Vantage"));
  });

  /**
   * L'adresse est insérée dans un attribut HTML monté à la main : une valeur
   * qui n'est pas une URL simple ne doit pas y entrer.
   */
  it("refuse une adresse douteuse et garde la colonne alignee", () => {
    const html = dayTooltipHtml(
      [
        { axisValue: "2026-08-09", seriesName: "Avec", value: 1 },
        { axisValue: "2026-08-09", seriesName: "Sans", value: 1 },
      ],
      { icon: (name) => (name === "Avec" ? '" onerror="alert(1)' : null) },
    );
    expect(html).not.toContain("onerror");
    expect(html).not.toContain("<img");
    expect(html.match(/<span style="display:inline-block/g)).toHaveLength(2);
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

  it("nomme le pays et sa part au lieu de la série sans nom", () => {
    const html = countryTooltipHtml("DE", 25, 100);
    expect(html).toContain("Allemagne");
    expect(html).toContain("25,0 %");
    expect(html).not.toContain("série");
  });

  it("annonce un pays sans relevé plutôt qu'une valeur vide", () => {
    expect(countryTooltipHtml("FR", Number.NaN, 100)).toContain("aucun téléchargement relevé");
  });
});

describe("fillZoom", () => {
  /** Un cadre aux proportions de la carte n'a rien à combler. */
  it("laisse la carte telle quelle quand le cadre a ses proportions", () => {
    expect(fillZoom(1000, 1000 / MAP_ASPECT)).toBeCloseTo(1, 2);
  });

  it("agrandit la carte dans un cadre plus large qu'elle", () => {
    // Le panneau de la vue dépliée : bien plus large que la carte, il laissait
    // sinon deux marges vides de part et d'autre.
    expect(fillZoom(1080, 410)).toBeGreaterThan(1.1);
  });

  it("ne rapetisse jamais la carte, et ne l'agrandit pas sans fin", () => {
    // Un cadre plus haut que large laisserait des bandes en haut et en bas,
    // mais les combler exigerait de rogner des continents entiers.
    expect(fillZoom(400, 600)).toBe(1);
    expect(fillZoom(4000, 100)).toBe(1.15);
    expect(fillZoom(0, 0)).toBe(1);
  });
});

describe("cellTooltipHtml", () => {
  it("nomme le chargeur et la version de jeu", () => {
    const html = cellTooltipHtml("fabric", "1.21", 1200);
    expect(html).toContain("fabric · 1.21");
    expect(html).not.toContain("série");
  });
});

/**
 * Les infobulles sont du HTML monté à la main, et tout ce qu'elles nomment vient
 * du réseau : titre de mod, nom de chargeur, code pays. Un seul chevron qui
 * passe, et le fragment devient une balise.
 */
describe("échappement des textes relevés", () => {
  it("neutralise les caractères qui ouvriraient une balise", () => {
    expect(escapeHtml(`<img src=x onerror="alert(1)">`)).toBe(
      "&lt;img src=x onerror=&quot;alert(1)&quot;&gt;",
    );
    expect(escapeHtml("a & b")).toBe("a &amp; b");
    expect(escapeHtml("l'un")).toBe("l&#39;un");
    expect(escapeHtml("rien à signaler")).toBe("rien à signaler");
  });

  it("échappe le nom d'un mod dans l'infobulle d'un axe de jours", () => {
    const html = dayTooltipHtml([
      { axisValue: "2026-08-10", seriesName: "<script>alert(1)</script>", value: 12 },
    ]);
    expect(html).not.toContain("<script>");
    expect(html).toContain("&lt;script&gt;");
  });

  it("échappe le chargeur et la version de jeu", () => {
    const html = cellTooltipHtml("<b>fabric", "1.21\"><script>", 10);
    expect(html).toContain("&lt;b&gt;fabric");
    expect(html).not.toContain("<script>");
  });

  it("échappe un code pays que la liste ne reconnaît pas", () => {
    const html = countryTooltipHtml("<img onerror=1>", 5, 10);
    expect(html).not.toContain("<img");
    expect(html).toContain("&lt;img");
  });

  it("laisse passer le marqueur, qui vient d'ECharts et non du réseau", () => {
    const html = dayTooltipHtml([
      { axisValue: "2026-08-10", seriesName: "Mod", value: 3, marker: "<span></span>" },
    ]);
    expect(html).toContain("<span></span>");
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
