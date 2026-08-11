<script lang="ts">
  import Chart from "../../charts/Chart.svelte";
  import { rankingOption } from "../../charts/multiseries";
  import { palette } from "../../charts/theme";
  import StatRow from "../../components/StatRow.svelte";
  import WorldMap from "../../components/WorldMap.svelte";
  import { compactNumber, countryLabel } from "../../format";
  import { theme } from "../../theme.svelte";
  import type { CountryTotal } from "../../types";

  import DetailShell from "./DetailShell.svelte";

  let { countries }: { countries: CountryTotal[] } = $props();

  const known = $derived(countries.filter((c) => c.country !== "??"));
  const unknown = $derived(countries.find((c) => c.country === "??"));
  const totalKnown = $derived(known.reduce((s, c) => s + c.downloads, 0));
  const top = $derived([...known].slice(0, 15).reverse());

  const option = $derived(
    rankingOption(
      top.map((c) => countryLabel(c.country)),
      top.map((c) => c.downloads),
      palette(theme.dark),
    ),
  );

  const leader = $derived(known[0]);
</script>

<DetailShell
  title="Origine des téléchargements"
  subtitle="{known.length} pays identifiés · données Modrinth uniquement"
>
  <StatRow
    stats={[
      { label: "Pays identifiés", value: String(known.length) },
      {
        label: "Téléchargements localisés",
        value: compactNumber(totalKnown),
      },
      {
        label: "Premier pays",
        value: leader ? countryLabel(leader.country) : "—",
        hint: leader
          ? `${compactNumber(leader.downloads)} · ${Math.round((leader.downloads / totalKnown) * 100)} % du localisé`
          : undefined,
      },
      {
        label: "Origine inconnue",
        value: unknown ? compactNumber(unknown.downloads) : "0",
        hint: "non représentés sur la carte",
      },
    ]}
  />

  <div class="grid">
    <div class="panel map">
      <WorldMap {countries} />
    </div>

    <div class="panel">
      <h2>Top 15</h2>
      <Chart {option} height={430} />
    </div>
  </div>

  <div class="panel wide">
    <h2>Tous les pays</h2>
    <table>
      <thead>
        <tr><th class="left">Pays</th><th>Code</th><th>Téléchargements</th><th>Part</th></tr>
      </thead>
      <tbody>
        {#each known as row (row.country)}
          <tr>
            <td class="left">{countryLabel(row.country)}</td>
            <td class="dim">{row.country}</td>
            <td>{compactNumber(row.downloads)}</td>
            <td>
              <div class="bar">
                <span style="width: {totalKnown ? (row.downloads / totalKnown) * 100 : 0}%"></span>
              </div>
              {totalKnown ? ((row.downloads / totalKnown) * 100).toFixed(1) : "0"} %
            </td>
          </tr>
        {/each}
        {#if unknown}
          <tr class="unknown">
            <td class="left">Origine inconnue</td>
            <td class="dim">??</td>
            <td>{compactNumber(unknown.downloads)}</td>
            <td class="dim">hors carte</td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>
</DetailShell>

<style>
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(0, 1fr);
    gap: 14px;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .wide {
    margin-top: 14px;
  }
  h2 {
    margin: 0 0 10px;
    font-size: 0.9rem;
    font-weight: 600;
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
  td:last-child {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .left {
    text-align: left;
  }
  .dim {
    color: var(--text-dim);
  }
  .bar {
    width: 110px;
    height: 6px;
    border-radius: 3px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .unknown td {
    color: var(--warn);
  }
  @media (max-width: 1100px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
