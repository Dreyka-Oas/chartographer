import { DARK, type Palette } from "./theme";

export function sparklineOption(values: number[], p: Palette = DARK) {
  return {
    grid: { left: 0, right: 0, top: 2, bottom: 2 },
    xAxis: { type: "category", show: false, data: values.map((_, i) => i) },
    yAxis: { type: "value", show: false },
    series: [
      {
        type: "line",
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 1.5, color: p.accent },
        areaStyle: { opacity: 0.18, color: p.accent },
        data: values,
      },
    ],
  };
}
