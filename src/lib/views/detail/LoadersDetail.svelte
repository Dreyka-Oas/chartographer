<script lang="ts">
  import Chart from "../../charts/Chart.svelte";
  import { heatmapOption } from "../../charts/heatmap";
  import { rankingOption } from "../../charts/multiseries";
  import { palette } from "../../charts/theme";
  import StatRow from "../../components/StatRow.svelte";
  import { compactNumber } from "../../format";
  import { theme } from "../../theme.svelte";
  import type { LoaderCell } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let { cells }: { cells: LoaderCell[] } = $props();

  function fold(key: (c: LoaderCell) => string) {
    const totals = new Map<string, number>();
    for (const c of cells) totals.set(key(c), (totals.get(key(c)) ?? 0) + c.downloads);
    return [...totals.entries()]
      .map(([name, downloads]) => ({ name, downloads }))
      .sort((a, b) => b.downloads - a.downloads);
  }

  const byLoader = $derived(fold((c) => c.loader));
  const byVersion = $derived(fold((c) => c.game_version));
  const total = $derived(byLoader.reduce((s, r) => s + r.downloads, 0));

  const heat = $derived(heatmapOption(cells, palette(theme.dark)));
  const loaderChart = $derived(
    rankingOption(
      [...byLoader].reverse().map((r) => r.name),
      [...byLoader].reverse().map((r) => r.downloads),
      palette(theme.dark),
      palette(theme.dark).modrinth,
    ),
  );
  const versionChart = $derived(
    rankingOption(
      [...byVersion].slice(0, 14).reverse().map((r) => r.name),
      [...byVersion].slice(0, 14).reverse().map((r) => r.downloads),
      palette(theme.dark),
      palette(theme.dark).curseforge,
    ),
  );
</script>

<DetailShell
  title="Versions de jeu et loaders"
  subtitle="{byVersion.length} versions · {byLoader.length} loaders · téléchargements Modrinth"
>
  <StatRow
    stats={[
      {
        label: "Loader dominant",
        value: byLoader[0]?.name ?? "—",
        hint: byLoader[0]
          ? `${Math.round((byLoader[0].downloads / total) * 100)} % des téléchargements`
          : undefined,
      },
      {
        label: "Version dominante",
        value: byVersion[0]?.name ?? "—",
        hint: byVersion[0] ? compactNumber(byVersion[0].downloads) : undefined,
      },
      { label: "Versions couvertes", value: String(byVersion.length) },
      { label: "Combinaisons", value: String(cells.length), hint: "version × loader" },
    ]}
  />

  {#if cells.length === 0}
    <p class="empty">Aucune version indexée. Lance une synchronisation.</p>
  {:else}
    <div class="panel">
      <h2>Carte de chaleur</h2>
      <p class="note">
        Une cellule compte le total d'une version de jeu croisée avec un loader. Une même
        publication qui vise trois versions et deux loaders alimente six cellules.
      </p>
      <Chart option={heat} height={Math.max(300, byLoader.length * 46 + 190)} />
    </div>

    <div class="grid">
      <div class="panel">
        <h2>Par loader</h2>
        <Chart option={loaderChart} height={Math.max(200, byLoader.length * 34 + 40)} />
      </div>
      <div class="panel">
        <h2>Par version de jeu</h2>
        <Chart option={versionChart} height={Math.max(200, Math.min(byVersion.length, 14) * 34 + 40)} />
      </div>
    </div>

    <div class="panel wide">
      <h2>Détail par loader</h2>
      <table>
        <thead>
          <tr><th class="left">Loader</th><th>Téléchargements</th><th>Part</th></tr>
        </thead>
        <tbody>
          {#each byLoader as row (row.name)}
            <tr>
              <td class="left">{row.name}</td>
              <td>{compactNumber(row.downloads)}</td>
              <td>{total ? ((row.downloads / total) * 100).toFixed(1) : "0"} %</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</DetailShell>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(380px, 1fr));
    gap: 14px;
    margin-top: 14px;
  }
  .wide {
    margin-top: 14px;
    max-width: 640px;
  }
  h2 {
    margin: 0 0 8px;
    font-family: var(--font-display);
    font-size: 0.98rem;
    font-weight: 600;
  }
  .note {
    margin: 0 0 10px;
    font-size: 0.78rem;
    color: var(--text-dim);
    max-width: 78ch;
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.88rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
  }
  th {
    text-align: right;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-weight: 500;
  }
  td {
    text-align: right;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .left {
    text-align: left;
  }
</style>
