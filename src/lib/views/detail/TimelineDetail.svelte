<script lang="ts">
  import Chart from "../../charts/Chart.svelte";
  import { foldSeriesTail, seriesColor, stackedProjectsOption } from "../../charts/multiseries";
  import { palette } from "../../charts/theme";
  import { timelineOption } from "../../charts/timeline";
  import Hint from "../../components/Hint.svelte";
  import RankedTable from "../../components/RankedTable.svelte";
  import StatRow from "../../components/StatRow.svelte";
  import Switch from "../../components/Switch.svelte";
  import { compactNumber, formatDay } from "../../format";
  import { dashboard } from "../../state.svelte";
  import { theme } from "../../theme.svelte";
  import type { Overview } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let { overview }: { overview: Overview } = $props();

  let mode = $state<"projects" | "platforms">("projects");
  let stacked = $state(true);

  /** Au-delà, la légende déborde et le rendu s'effondre : la queue est repliée. */
  const MAX_SERIES = 12;

  const active = $derived(
    [...overview.per_project]
      .filter((p) => p.spark.some((v) => v > 0))
      .sort(
        (a, b) => b.spark.reduce((s, v) => s + v, 0) - a.spark.reduce((s, v) => s + v, 0),
      )
      .map((p) => ({ name: p.title, values: p.spark })),
  );
  const series = $derived(foldSeriesTail(active, MAX_SERIES));
  const folded = $derived(Math.max(0, active.length - MAX_SERIES));

  const option = $derived(
    mode === "projects"
      ? stackedProjectsOption(overview.days, series, palette(theme.dark), stacked)
      : timelineOption(overview.timeline, stacked, palette(theme.dark)),
  );

  const total = $derived(overview.timeline.reduce((s, d) => s + d.modrinth + d.curseforge, 0));
  const best = $derived(
    overview.timeline.reduce(
      (acc, d) => (d.modrinth + d.curseforge > acc.total ? { day: d.day, total: d.modrinth + d.curseforge } : acc),
      { day: "", total: 0 },
    ),
  );
  const average = $derived(overview.timeline.length ? Math.round(total / overview.timeline.length) : 0);

  const top = $derived(
    [...overview.timeline]
      .map((d) => ({ ...d, total: d.modrinth + d.curseforge }))
      .sort((a, b) => b.total - a.total)
      .slice(0, 12),
  );
</script>

<DetailShell
  title="Téléchargements par jour"
  subtitle="{overview.days.length} jours · {active.length} projets actifs{folded > 0
    ? ` · les ${folded} plus petits regroupés en une courbe`
    : ''}"
>
  {#snippet actions()}
    <div class="switch">
      <button class:active={mode === "projects"} onclick={() => (mode = "projects")}>
        Par mod
      </button>
      <button class:active={mode === "platforms"} onclick={() => (mode = "platforms")}>
        Par plateforme
      </button>
    </div>
    <Switch
      bind:checked={stacked}
      label="Empiler"
      title={mode === "projects"
        ? "Les mods s'additionnent en une pile, ou se superposent en courbes"
        : "Les deux plateformes s'additionnent, ou se comparent niveau à niveau"}
    />
  {/snippet}

  <StatRow
    stats={[
      { label: "Total sur la période", value: compactNumber(total) },
      { label: "Moyenne par jour", value: compactNumber(average) },
      {
        label: "Meilleure journée",
        value: compactNumber(best.total),
        hint: best.day ? formatDay(best.day) : "—",
      },
      { label: "Mods actifs", value: String(series.length), hint: "au moins un téléchargement" },
    ]}
  />

  <div class="chart">
    <Chart {option} height={480} morph />
  </div>

  <div class="split">
    <div class="panel">
      <h2>
        Meilleures journées
        <Hint
          text="Les douze jours les mieux servis de la période affichée, du plus fort au plus faible. Les trois premiers sont marqués d'un rang coloré. Changer les dates change ce classement."
        />
      </h2>
      <RankedTable
        columns={[
          { label: "Jour", align: "left" },
          { label: "Modrinth" },
          { label: "CurseForge" },
          { label: "Total" },
        ]}
        rows={top}
        key={(row) => row.day}
      >
        {#snippet cells(row, i)}
          <td class="left">{formatDay(row.day)}</td>
          <td>{compactNumber(row.modrinth)}</td>
          <td>{compactNumber(row.curseforge)}</td>
          <td><b class:lead={i === 0}>{compactNumber(row.total)}</b></td>
        {/snippet}
      </RankedTable>
    </div>

    <div class="panel">
      <h2>
        Contribution par mod
        <Hint
          text="Ce que chaque mod a rapporté de téléchargements sur la période, et la part que cela représente dans le total. La pastille reprend sa couleur dans le graphique ci-dessus. Cliquer sur une ligne ouvre la fiche du mod."
        />
      </h2>
      <RankedTable
        columns={[{ label: "Mod", align: "left" }, { label: "Période" }, { label: "Part" }]}
        rows={series}
        key={(s) => s.name}
        onselect={(s) => {
          const found = overview.per_project.find((p) => p.title === s.name);
          if (found) dashboard.openProject(found);
        }}
      >
        {#snippet cells(s, i)}
          {@const sum = s.values.reduce((a, v) => a + v, 0)}
          <td class="left">
            <span class="dot" style="background: {seriesColor(i)}"></span>
            {s.name}
          </td>
          <td>{compactNumber(sum)}</td>
          <td>{total ? Math.round((sum / total) * 100) : 0} %</td>
        {/snippet}
      </RankedTable>
    </div>
  </div>
</DetailShell>

<style>
  .switch {
    display: flex;
    gap: 4px;
  }
  .switch button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 7px;
    padding: 5px 12px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .switch button.active,
  .switch button:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .chart {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px;
  }
  .split {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
    gap: 14px;
    margin-top: 14px;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 10px;
    font-size: 0.9rem;
    font-weight: 600;
  }
  /* Filets, alignements et survol vivent dans `RankedTable` ; ne restent ici
   * que les marques propres à ces deux tableaux. */
  .lead {
    color: var(--accent);
  }
  .dot {
    display: inline-block;
    width: 9px;
    height: 9px;
    margin-right: 8px;
    border-radius: 999px;
    vertical-align: middle;
  }
</style>
