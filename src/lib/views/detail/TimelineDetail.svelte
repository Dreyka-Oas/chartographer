<script lang="ts">
  import Chart from "../../charts/Chart.svelte";
  import { foldSeriesTail, seriesColor, stackedProjectsOption } from "../../charts/multiseries";
  import { palette } from "../../charts/theme";
  import { timelineOption } from "../../charts/timeline";
  import Hint from "../../components/Hint.svelte";
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

  /**
   * Or, argent, bronze pour les trois meilleures journées. Ces teintes sont
   * écrites en clair plutôt que prises au thème : elles ne veulent rien dire
   * d'autre qu'un rang, et doivent se lire pareil en clair comme en sombre.
   */
  const PODIUM = ["#d4a72c", "#9aa4ad", "#b97a45"];

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
      <table>
        <thead>
          <tr><th class="left">Jour</th><th>Modrinth</th><th>CurseForge</th><th>Total</th></tr>
        </thead>
        <tbody>
          {#each top as row, i (row.day)}
            <tr>
              <td class="left">
                <span class="rank" class:podium={i < 3} style="--rank: {PODIUM[i] ?? ''}">
                  {i + 1}
                </span>
                {formatDay(row.day)}
              </td>
              <td>{compactNumber(row.modrinth)}</td>
              <td>{compactNumber(row.curseforge)}</td>
              <td><b class:lead={i === 0}>{compactNumber(row.total)}</b></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="panel">
      <h2>
        Contribution par mod
        <Hint
          text="Ce que chaque mod a rapporté de téléchargements sur la période, et la part que cela représente dans le total. La pastille reprend sa couleur dans le graphique ci-dessus. Cliquer sur une ligne ouvre la fiche du mod."
        />
      </h2>
      <table>
        <thead>
          <tr><th class="left">Mod</th><th>Période</th><th>Part</th></tr>
        </thead>
        <tbody>
          {#each series as s, i (s.name)}
            {@const sum = s.values.reduce((a, v) => a + v, 0)}
            <tr
              onclick={() => {
                const found = overview.per_project.find((p) => p.title === s.name);
                if (found) dashboard.openProject(found);
              }}
            >
              <td class="left">
                <span class="dot" style="background: {seriesColor(i)}"></span>
                {s.name}
              </td>
              <td>{compactNumber(sum)}</td>
              <td>{total ? Math.round((sum / total) * 100) : 0} %</td>
            </tr>
          {/each}
        </tbody>
      </table>
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
  .rank {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    margin-right: 8px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-dim);
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
    /* Le rang est déjà donné par l'ordre des lignes : la couleur le redit,
     * elle ne le porte pas seule. */
    vertical-align: middle;
  }
  .rank.podium {
    background: color-mix(in srgb, var(--rank) 22%, transparent);
    color: var(--rank);
    font-weight: 600;
  }
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
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
  }
  th {
    text-align: right;
    padding: 5px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-weight: 500;
  }
  td {
    text-align: right;
    padding: 5px 8px;
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .left {
    text-align: left;
  }
  tbody tr:hover {
    background: var(--surface-2);
  }
  .panel:last-child tbody tr {
    cursor: pointer;
  }
</style>
