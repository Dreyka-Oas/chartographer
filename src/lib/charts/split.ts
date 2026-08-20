import type { ProjectSummary } from "../types";
import { axisStyle, BASE_GRID, DARK, tooltip, type Palette, valueAxis } from "./theme";

export function splitOption(projects: ProjectSummary[], p: Palette = DARK) {
  const rows = [...projects]
    .sort(
      (a, b) =>
        b.modrinth_downloads +
        b.curseforge_downloads -
        (a.modrinth_downloads + a.curseforge_downloads),
    )
    .slice(0, 15)
    .reverse();

  return {
    /*
     * La marge de droite tient le dernier libellé de l'axe, qui est centré sous
     * sa graduation et déborde donc de sa moitié. À 16 pixels, `200,0 k` se
     * faisait couper sa dernière lettre.
     */
    grid: { ...BASE_GRID, left: 140, right: 34 },
    tooltip: { trigger: "axis", axisPointer: { type: "shadow" }, ...tooltip(p) },
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: p.textDim }, top: 0 },
    xAxis: valueAxis(p),
    /*
     * Le nom du mod est borné. `containLabel` élargit la marge jusqu'à contenir
     * le plus long : un titre de soixante caractères prenait alors les trois
     * quarts du panneau, et les barres se réduisaient à des traits.
     */
    yAxis: {
      type: "category",
      data: rows.map((r) => r.title),
      ...axisStyle(p),
      /*
       * Chaque barre porte son nom. Serré, ECharts n'en écrivait qu'un sur
       * deux : la moitié des barres n'était rattachée à aucun mod, ce qui les
       * rendait illisibles plutôt que compactes.
       */
      axisLabel: {
        color: p.textDim,
        interval: 0,
        fontSize: 11,
        width: 130,
        overflow: "truncate",
        ellipsis: "…",
      },
    },
    series: [
      {
        name: "Modrinth",
        type: "bar",
        stack: "total",
        itemStyle: { color: p.modrinth },
        data: rows.map((r) => r.modrinth_downloads),
      },
      {
        name: "CurseForge",
        type: "bar",
        stack: "total",
        itemStyle: { color: p.curseforge },
        data: rows.map((r) => r.curseforge_downloads),
      },
    ],
  };
}
