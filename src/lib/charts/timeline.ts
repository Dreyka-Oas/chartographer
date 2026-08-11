import type { TimelinePoint } from "../types";
import { AXIS_STYLE, BASE_GRID, COLORS, TOOLTIP } from "./theme";

export function timelineOption(points: TimelinePoint[], stacked: boolean) {
  const stack = stacked ? "downloads" : undefined;
  return {
    grid: BASE_GRID,
    tooltip: { trigger: "axis", ...TOOLTIP },
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: COLORS.textDim }, top: 0 },
    xAxis: { type: "category", data: points.map((p) => p.day), ...AXIS_STYLE },
    yAxis: { type: "value", ...AXIS_STYLE },
    dataZoom: [
      { type: "inside", start: 0, end: 100 },
      {
        type: "slider",
        height: 20,
        bottom: 8,
        borderColor: COLORS.grid,
        textStyle: { color: COLORS.textDim },
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
        itemStyle: { color: COLORS.modrinth },
        data: points.map((p) => p.modrinth),
      },
      {
        name: "CurseForge",
        type: "line",
        stack,
        smooth: true,
        showSymbol: false,
        areaStyle: { opacity: 0.25 },
        itemStyle: { color: COLORS.curseforge },
        data: points.map((p) => p.curseforge),
      },
    ],
  };
}
