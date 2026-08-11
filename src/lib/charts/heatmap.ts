import type { LoaderCell } from "../types";
import { AXIS_STYLE, COLORS, TOOLTIP } from "./theme";

/** Trie les versions de jeu par ordre numérique croissant plutôt qu'alphabétique. */
function sortGameVersions(values: string[]): string[] {
  return [...values].sort((a, b) => {
    const pa = a.split(".").map((n) => Number.parseInt(n, 10) || 0);
    const pb = b.split(".").map((n) => Number.parseInt(n, 10) || 0);
    for (let i = 0; i < Math.max(pa.length, pb.length); i += 1) {
      const diff = (pa[i] ?? 0) - (pb[i] ?? 0);
      if (diff !== 0) return diff;
    }
    return 0;
  });
}

export function heatmapOption(cells: LoaderCell[]) {
  const gameVersions = sortGameVersions([...new Set(cells.map((c) => c.game_version))]);
  const loaders = [...new Set(cells.map((c) => c.loader))].sort();
  const max = cells.reduce((acc, c) => Math.max(acc, c.downloads), 0);

  return {
    grid: { left: 90, right: 20, top: 16, bottom: 70, containLabel: true },
    tooltip: { position: "top", ...TOOLTIP },
    xAxis: {
      type: "category",
      data: gameVersions,
      ...AXIS_STYLE,
      axisLabel: { color: COLORS.textDim, rotate: 45 },
    },
    yAxis: { type: "category", data: loaders, ...AXIS_STYLE },
    visualMap: {
      min: 0,
      max,
      calculable: false,
      orient: "horizontal",
      left: "center",
      bottom: 0,
      textStyle: { color: COLORS.textDim },
      inRange: { color: ["#14181d", COLORS.accent] },
    },
    series: [
      {
        type: "heatmap",
        data: cells.map((c) => [
          gameVersions.indexOf(c.game_version),
          loaders.indexOf(c.loader),
          c.downloads,
        ]),
        emphasis: { itemStyle: { borderColor: COLORS.text, borderWidth: 1 } },
      },
    ],
  };
}
