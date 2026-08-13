import type { TimelinePoint } from "../types";
import { drawOrder, stackValues } from "./multiseries";
import { axisStyle, BASE_GRID, DARK, dayAxis, dayTooltip, type Palette } from "./theme";

export function timelineOption(points: TimelinePoint[], stacked: boolean, p: Palette = DARK) {
  const axis = axisStyle(p);
  const modrinth = points.map((x) => x.modrinth);
  const curseforge = points.map((x) => x.curseforge);
  // Voir `stackValues` : l'empilement est calculé ici pour que le basculement
  // soit un changement de valeurs, seule chose qu'ECharts anime.
  const [lower, upper] = stacked ? stackValues([modrinth, curseforge]) : [modrinth, curseforge];
  return {
    // Voir `stackedProjectsOption` : l'`id` des séries permet la transition, et
    // nommer une courbe d'accélération l'annulerait.
    animationDurationUpdate: 700,
    grid: BASE_GRID,
    tooltip: dayTooltip(p, undefined, true),
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: p.textDim }, top: 0 },
    xAxis: dayAxis(
      points.map((x) => x.day),
      p,
    ),
    yAxis: { type: "value", ...axis },
    dataZoom: [
      { type: "inside", start: 0, end: 100 },
      {
        type: "slider",
        height: 20,
        bottom: 8,
        borderColor: p.grid,
        textStyle: { color: p.textDim },
      },
    ],
    series: drawOrder(
      [
        {
          id: "platform:modrinth",
          name: "Modrinth",
          type: "line",
          smooth: true,
          showSymbol: false,
          // Opaque une fois empilée, sans quoi les deux couches se mélangeraient
          // et la bande du bas ne serait plus à la couleur de sa plateforme.
          areaStyle: { opacity: stacked ? 1 : 0.25 },
          itemStyle: { color: p.modrinth },
          data: modrinth.map((own, day) => ({ value: lower[day], own })),
        },
        {
          id: "platform:curseforge",
          name: "CurseForge",
          type: "line",
          smooth: true,
          showSymbol: false,
          // Opaque une fois empilée, sans quoi les deux couches se mélangeraient
          // et la bande du bas ne serait plus à la couleur de sa plateforme.
          areaStyle: { opacity: stacked ? 1 : 0.25 },
          itemStyle: { color: p.curseforge },
          data: curseforge.map((own, day) => ({ value: upper[day], own })),
        },
      ],
      stacked,
    ),
  };
}
