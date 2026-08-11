import type { ProjectSummary } from "../types";
import { AXIS_STYLE, BASE_GRID, COLORS, TOOLTIP } from "./theme";

export function splitOption(projects: ProjectSummary[]) {
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
    grid: { ...BASE_GRID, left: 140 },
    tooltip: { trigger: "axis", axisPointer: { type: "shadow" }, ...TOOLTIP },
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: COLORS.textDim }, top: 0 },
    xAxis: { type: "value", ...AXIS_STYLE },
    yAxis: { type: "category", data: rows.map((r) => r.title), ...AXIS_STYLE },
    series: [
      {
        name: "Modrinth",
        type: "bar",
        stack: "total",
        itemStyle: { color: COLORS.modrinth },
        data: rows.map((r) => r.modrinth_downloads),
      },
      {
        name: "CurseForge",
        type: "bar",
        stack: "total",
        itemStyle: { color: COLORS.curseforge },
        data: rows.map((r) => r.curseforge_downloads),
      },
    ],
  };
}
