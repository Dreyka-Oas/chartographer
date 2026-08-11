import type { RevenuePoint } from "../types";
import { AXIS_STYLE, BASE_GRID, COLORS, TOOLTIP } from "./theme";

export function revenueOption(points: RevenuePoint[]) {
  const daily = points.map((p) => Number.parseFloat(p.amount) || 0);
  let running = 0;
  const cumulative = daily.map((v) => {
    running += v;
    return Number(running.toFixed(4));
  });

  return {
    grid: BASE_GRID,
    tooltip: { trigger: "axis", ...TOOLTIP },
    legend: { data: ["Journalier", "Cumulé"], textStyle: { color: COLORS.textDim }, top: 0 },
    xAxis: { type: "category", data: points.map((p) => p.day), ...AXIS_STYLE },
    yAxis: [
      { type: "value", ...AXIS_STYLE },
      { type: "value", ...AXIS_STYLE },
    ],
    series: [
      { name: "Journalier", type: "bar", itemStyle: { color: COLORS.accent }, data: daily },
      {
        name: "Cumulé",
        type: "line",
        yAxisIndex: 1,
        smooth: true,
        showSymbol: false,
        itemStyle: { color: COLORS.curseforge },
        data: cumulative,
      },
    ],
  };
}
