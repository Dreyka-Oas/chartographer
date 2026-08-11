import type { ProjectSummary } from "../types";
import { axisStyle, BASE_GRID, DARK, tooltip, type Palette } from "./theme";

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
  const axis = axisStyle(p);

  return {
    grid: { ...BASE_GRID, left: 140 },
    tooltip: { trigger: "axis", axisPointer: { type: "shadow" }, ...tooltip(p) },
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: p.textDim }, top: 0 },
    xAxis: { type: "value", ...axis },
    yAxis: { type: "category", data: rows.map((r) => r.title), ...axis },
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
