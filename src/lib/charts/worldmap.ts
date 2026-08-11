import type { CountryTotal } from "../types";
import { COLORS, TOOLTIP } from "./theme";

/** Le code `??` agrège `XX` et la chaîne vide côté Rust : il n'a pas de géométrie. */
export function worldMapOption(countries: CountryTotal[]) {
  const mapped = countries.filter((c) => c.country !== "??");
  const max = mapped.reduce((acc, c) => Math.max(acc, c.downloads), 0);

  return {
    tooltip: { trigger: "item", ...TOOLTIP },
    visualMap: {
      min: 0,
      max: Math.max(max, 1),
      left: 12,
      bottom: 12,
      calculable: true,
      textStyle: { color: COLORS.textDim },
      inRange: { color: ["#1b2027", COLORS.accent, COLORS.curseforge] },
    },
    series: [
      {
        type: "map",
        map: "world",
        roam: true,
        itemStyle: { areaColor: "#161b21", borderColor: COLORS.grid },
        emphasis: { label: { show: false }, itemStyle: { areaColor: COLORS.accent } },
        nameProperty: "iso_a2",
        data: mapped.map((c) => ({ name: c.country, value: c.downloads })),
      },
    ],
  };
}
