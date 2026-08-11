import type { PayoutPoint } from "../types";
import { axisStyle, BASE_GRID, DARK, monthAxis, tooltip, type Palette } from "./theme";

/**
 * Échéancier de reversement. Les échéances déjà mûres et celles à venir
 * portent deux couleurs distinctes, et une ligne de repère marque aujourd'hui.
 */
export function scheduleOption(points: PayoutPoint[], p: Palette = DARK) {
  const axis = axisStyle(p);
  const firstFuture = points.findIndex((x) => x.future);

  return {
    grid: { ...BASE_GRID, top: 30, bottom: 60 },
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      ...tooltip(p),
      valueFormatter: (v: number) => `${v.toFixed(2)} $`,
    },
    xAxis: monthAxis(
      points.map((x) => x.date.slice(0, 7)),
      p,
    ),
    yAxis: { type: "value", ...axis },
    series: [
      {
        type: "bar",
        data: points.map((x) => ({
          value: Number.parseFloat(x.amount) || 0,
          itemStyle: { color: x.future ? p.curseforge : p.accent, opacity: x.future ? 0.72 : 1 },
        })),
        barMaxWidth: 34,
        itemStyle: { borderRadius: [3, 3, 0, 0] },
        markLine:
          firstFuture > 0
            ? {
                silent: true,
                symbol: "none",
                lineStyle: { color: p.textDim, type: "dashed", width: 1 },
                label: { formatter: "aujourd'hui", color: p.textDim, fontSize: 10 },
                data: [{ xAxis: firstFuture - 0.5 }],
              }
            : undefined,
      },
    ],
  };
}
