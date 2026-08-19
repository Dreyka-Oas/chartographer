import { formatMoney } from "../format";
import type { RevenuePoint } from "../types";
import { BASE_GRID, DARK, dayAxis, dayTooltip, type Palette, valueAxis } from "./theme";

export function revenueOption(points: RevenuePoint[], p: Palette = DARK) {
  const daily = points.map((x) => Number.parseFloat(x.amount) || 0);
  let running = 0;
  const cumulative = daily.map((v) => {
    running += v;
    return Number(running.toFixed(4));
  });

  return {
    grid: BASE_GRID,
    tooltip: dayTooltip(p, { format: (value) => formatMoney(String(value)) }),
    legend: { data: ["Journalier", "Cumulé"], textStyle: { color: p.textDim }, top: 0 },
    xAxis: dayAxis(
      points.map((x) => x.day),
      p,
    ),
    yAxis: [valueAxis(p), valueAxis(p)],
    series: [
      { name: "Journalier", type: "bar", itemStyle: { color: p.accent }, data: daily },
      {
        name: "Cumulé",
        type: "line",
        yAxisIndex: 1,
        smooth: true,
        showSymbol: false,
        itemStyle: { color: p.curseforge },
        data: cumulative,
      },
    ],
  };
}
