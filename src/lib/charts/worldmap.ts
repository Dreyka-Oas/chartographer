import type { CountryTotal } from "../types";
import { DARK, tooltip, type Palette } from "./theme";

/** Le code `??` agrège `XX` et la chaîne vide côté Rust : il n'a pas de géométrie. */
export function worldMapOption(countries: CountryTotal[], p: Palette = DARK) {
  const mapped = countries.filter((c) => c.country !== "??");
  const max = mapped.reduce((acc, c) => Math.max(acc, c.downloads), 0);

  return {
    tooltip: { trigger: "item", ...tooltip(p) },
    visualMap: {
      min: 0,
      max: Math.max(max, 1),
      left: 12,
      bottom: 12,
      calculable: true,
      textStyle: { color: p.textDim },
      inRange: { color: [p.empty, p.accent, p.curseforge] },
    },
    series: [
      {
        type: "map",
        map: "world",
        roam: true,
        itemStyle: { areaColor: p.empty, borderColor: p.grid },
        emphasis: { label: { show: false }, itemStyle: { areaColor: p.accent } },
        nameProperty: "iso_a2",
        data: mapped.map((c) => ({ name: c.country, value: c.downloads })),
      },
    ],
  };
}
