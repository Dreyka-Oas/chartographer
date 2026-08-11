import { axisStyle, BASE_GRID, DARK, dayAxis, dayTooltip, tooltip, type Palette } from "./theme";

export interface NamedSeries {
  name: string;
  values: number[];
}

/**
 * Palette catégorielle pour les séries par projet. Les teintes sont espacées
 * en luminance autant qu'en couleur, pour rester distinguables en clair
 * comme en sombre et lisibles en cas de daltonisme rouge-vert.
 */
export const SERIES_COLORS = [
  "#5ac8a8",
  "#f16436",
  "#6aa9ff",
  "#e0a458",
  "#b98cff",
  "#3fbfbf",
  "#ef6f9e",
  "#8fc75a",
  "#c58a5a",
  "#7f8fa6",
];

export function seriesColor(index: number): string {
  return SERIES_COLORS[index % SERIES_COLORS.length];
}

/** Aire empilée par projet, sur un axe de jours déjà dense. */
export function stackedProjectsOption(
  days: string[],
  series: NamedSeries[],
  p: Palette = DARK,
  stacked = true,
) {
  const axis = axisStyle(p);
  return {
    grid: { ...BASE_GRID, top: 40, bottom: 72 },
    tooltip: { ...dayTooltip(p), order: "valueDesc" },
    legend: {
      type: "scroll",
      data: series.map((s) => s.name),
      textStyle: { color: p.textDim },
      top: 0,
    },
    xAxis: dayAxis(days, p),
    yAxis: { type: "value", ...axis },
    dataZoom: [
      { type: "inside", start: 0, end: 100 },
      {
        type: "slider",
        height: 22,
        bottom: 10,
        borderColor: p.grid,
        textStyle: { color: p.textDim },
      },
    ],
    series: series.map((s, i) => ({
      name: s.name,
      type: "line",
      stack: stacked ? "total" : undefined,
      smooth: true,
      showSymbol: false,
      lineStyle: { width: stacked ? 1 : 2 },
      areaStyle: stacked ? { opacity: 0.55 } : undefined,
      itemStyle: { color: seriesColor(i) },
      data: s.values,
    })),
  };
}

/** Barres horizontales simples, triées par l'appelant. */
export function rankingOption(
  labels: string[],
  values: number[],
  p: Palette = DARK,
  color = p.accent,
) {
  const axis = axisStyle(p);
  return {
    grid: { left: 8, right: 24, top: 8, bottom: 8, containLabel: true },
    tooltip: { trigger: "axis", axisPointer: { type: "shadow" }, ...tooltip(p) },
    xAxis: { type: "value", ...axis },
    yAxis: { type: "category", data: labels, ...axis },
    series: [
      {
        type: "bar",
        itemStyle: { color, borderRadius: [0, 4, 4, 0] },
        data: values,
      },
    ],
  };
}
