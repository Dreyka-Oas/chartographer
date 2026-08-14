import { axisStyle, BASE_GRID, DARK, dayAxis, dayTooltip, tooltip, type Palette } from "./theme";

export interface NamedSeries {
  name: string;
  values: number[];
  /**
   * Logo du mod, quand la plateforme en publie un. Une pastille de couleur dit
   * quelle courbe est laquelle, mais douze pastilles se ressemblent : le logo
   * est ce que l'auteur reconnaît d'un coup d'œil.
   */
  icon?: string | null;
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

/**
 * Garde les `max` premières séries et replie toutes les autres en une seule,
 * jour par jour. Au-delà d'une douzaine de courbes la légende devient illisible
 * et le rendu s'effondre ; le total reste juste puisque la queue est sommée.
 * L'appelant reçoit la série de repli nommée, à lui de l'annoncer.
 */
export function foldSeriesTail(series: NamedSeries[], max: number): NamedSeries[] {
  if (series.length <= max) return series;
  const head = series.slice(0, max);
  const tail = series.slice(max);
  const length = series[0]?.values.length ?? 0;
  const merged = new Array<number>(length).fill(0);
  for (const item of tail) {
    for (let i = 0; i < length; i += 1) merged[i] += item.values[i] ?? 0;
  }
  return [...head, { name: `${tail.length} autres mods`, values: merged }];
}

/**
 * Empile les séries jour par jour : la valeur rendue pour la série `i` est la
 * somme des séries `0..i`, ce que dessinerait un empilement.
 *
 * Le calcul est fait ici plutôt que laissé à ECharts, et ce n'est pas un
 * caprice : l'option `stack` se règle en amont du rendu, si bien que la
 * décocher refait le tracé d'un bloc, sans transition. En ne changeant que des
 * valeurs, on retombe sur ce qu'ECharts sait animer.
 */
export function stackValues(rows: number[][]): number[][] {
  const running: number[] = [];
  return rows.map((row) =>
    row.map((value, day) => {
      running[day] = (running[day] ?? 0) + value;
      return running[day];
    }),
  );
}

/**
 * Ordre de tracé des séries empilées : la plus haute en premier.
 *
 * Chaque aire est remplie depuis la ligne du bas jusqu'à son cumul, et non
 * entre deux cumuls comme le ferait un empilement d'ECharts. Peintes dans
 * l'ordre, les grandes recouvriraient les petites ; peintes de la plus haute à
 * la plus basse, chaque bande laisse voir la couleur de la série à laquelle
 * elle revient — le même dessin qu'un empilement.
 */
export function drawOrder<T>(items: T[], stacked: boolean): T[] {
  return stacked ? [...items].reverse() : items;
}

/** Aire empilée par projet, sur un axe de jours déjà dense. */
export function stackedProjectsOption(
  days: string[],
  series: NamedSeries[],
  p: Palette = DARK,
  stacked = true,
) {
  const axis = axisStyle(p);
  const drawn = stacked ? stackValues(series.map((s) => s.values)) : series.map((s) => s.values);
  // Le tooltip ne reçoit que des noms de séries : le logo se retrouve par là.
  const icons = new Map(series.map((s) => [s.name, s.icon ?? null]));
  return {
    /*
     * Le basculement se joue sur les mêmes séries, que leur `id` fait
     * retrouver d'un état à l'autre : ECharts interpole alors les tracés au
     * lieu de les refaire.
     *
     * Aucune courbe d'accélération n'est nommée ici : mesuré, `cubicOut`
     * annule la transition dans ECharts 6 — le tracé arrive à destination dès
     * la première image. Le réglage par défaut, lui, glisse bien.
     *
     * Le tri du tooltip est laissé au formateur, qui seul connaît la valeur
     * propre de chaque mod derrière le cumul dessiné.
     */
    animationDurationUpdate: 700,
    grid: { ...BASE_GRID, top: 40, bottom: 72 },
    tooltip: dayTooltip(p, { sorted: true, icon: (name) => icons.get(name) }),
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
    series: drawOrder(
      series.map((s, i) => ({
        id: `mod:${s.name}`,
        name: s.name,
        type: "line",
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 1.5 },
        /*
         * Empilées, les aires sont opaques : c'est la seule façon pour qu'une
         * bande montre la couleur du mod auquel elle revient, puisqu'elles
         * sont peintes depuis la ligne du bas et se recouvrent. Superposées,
         * elles s'effacent presque : garder des remplissages pleins noierait
         * les petits mods sous le plus gros, et ôterait tout intérêt à
         * désempiler.
         */
        areaStyle: { opacity: stacked ? 1 : 0.06 },
        itemStyle: { color: seriesColor(i) },
        data: s.values.map((own, day) => ({ value: drawn[i][day], own })),
      })),
      stacked,
    ),
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
