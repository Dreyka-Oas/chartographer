import { formatMoney } from "../format";
import type { PayoutPoint } from "../types";
import { BASE_GRID, DARK, monthAxis, tooltip, type Palette, valueAxis } from "./theme";

/**
 * Échéancier de reversement. Les échéances déjà mûres et celles à venir
 * portent deux couleurs distinctes, et une ligne de repère marque aujourd'hui.
 */
export function scheduleOption(points: PayoutPoint[], p: Palette = DARK) {
  const firstFuture = points.findIndex((x) => x.future);

  return {
    grid: { ...BASE_GRID, top: 30, bottom: 60 },
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      ...tooltip(p),
      // Les montants arrivent en dollars ; `formatMoney` les convertit dans la
      // devise choisie. L'écrire ici en dollars affichait une somme que ne
      // reconnaissait aucune des cartes de la même page.
      valueFormatter: (v: number) => formatMoney(String(v)),
    },
    xAxis: monthAxis(
      points.map((x) => x.date.slice(0, 7)),
      p,
    ),
    yAxis: valueAxis(p),
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
