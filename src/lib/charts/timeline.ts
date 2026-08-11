import type { TimelinePoint } from "../types";
import { axisStyle, BASE_GRID, DARK, tooltip, type Palette } from "./theme";

export function timelineOption(points: TimelinePoint[], stacked: boolean, p: Palette = DARK) {
  const stack = stacked ? "downloads" : undefined;
  const axis = axisStyle(p);
  return {
    grid: BASE_GRID,
    tooltip: { trigger: "axis", ...tooltip(p) },
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: p.textDim }, top: 0 },
    xAxis: { type: "category", data: points.map((x) => x.day), ...axis },
    yAxis: { type: "value", ...axis },
    dataZoom: [
      { type: "inside", start: 0, end: 100 },
      {
        type: "slider",
        height: 20,
        bottom: 8,
        borderColor: p.grid,
        textStyle: { color: p.textDim },
      },
    ],
    series: [
      {
        name: "Modrinth",
        type: "line",
        stack,
        smooth: true,
        showSymbol: false,
        areaStyle: { opacity: 0.25 },
        itemStyle: { color: p.modrinth },
        data: points.map((x) => x.modrinth),
      },
      {
        name: "CurseForge",
        type: "line",
        stack,
        smooth: true,
        showSymbol: false,
        areaStyle: { opacity: 0.25 },
        itemStyle: { color: p.curseforge },
        data: points.map((x) => x.curseforge),
      },
    ],
  };
}
