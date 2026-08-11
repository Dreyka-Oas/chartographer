<script lang="ts">
  import Chart from "../../charts/Chart.svelte";
  import { splitOption } from "../../charts/split";
  import { palette } from "../../charts/theme";
  import StatRow from "../../components/StatRow.svelte";
  import { compactNumber } from "../../format";
  import { dashboard } from "../../state.svelte";
  import { theme } from "../../theme.svelte";
  import type { ProjectSummary } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let { projects }: { projects: ProjectSummary[] } = $props();

  const total = (p: ProjectSummary) => p.modrinth_downloads + p.curseforge_downloads;
  const share = (p: ProjectSummary) => (total(p) === 0 ? 0 : (p.modrinth_downloads / total(p)) * 100);

  const modrinth = $derived(projects.reduce((s, p) => s + p.modrinth_downloads, 0));
  const curseforge = $derived(projects.reduce((s, p) => s + p.curseforge_downloads, 0));
  const grand = $derived(modrinth + curseforge);

  const bothPlatforms = $derived(
    projects.filter((p) => p.modrinth_id !== null && p.curseforge_id !== null),
  );
  const rows = $derived([...projects].sort((a, b) => total(b) - total(a)));
  const option = $derived(splitOption(projects, palette(theme.dark)));

  /** Le mod dont l'écart entre plateformes est le plus marqué. */
  const mostLopsided = $derived(
    bothPlatforms.reduce<ProjectSummary | null>(
      (acc, p) => (acc === null || Math.abs(share(p) - 50) > Math.abs(share(acc) - 50) ? p : acc),
      null,
    ),
  );
</script>

<DetailShell
  title="Modrinth contre CurseForge"
  subtitle="{bothPlatforms.length} mods présents sur les deux plateformes, sur {projects.length}"
>
  <StatRow
    stats={[
      {
        label: "Part Modrinth",
        value: `${grand ? Math.round((modrinth / grand) * 100) : 0} %`,
        hint: compactNumber(modrinth),
      },
      {
        label: "Part CurseForge",
        value: `${grand ? Math.round((curseforge / grand) * 100) : 0} %`,
        hint: compactNumber(curseforge),
      },
      { label: "Toutes plateformes", value: compactNumber(grand) },
      {
        label: "Écart le plus marqué",
        value: mostLopsided ? `${Math.round(share(mostLopsided))} %` : "—",
        hint: mostLopsided ? `${mostLopsided.title}, part Modrinth` : undefined,
      },
    ]}
  />

  <div class="panel">
    <h2>
      Quinze premiers par volume
      {#if rows.length > 15}<span class="note">{rows.length - 15} autres dans le tableau</span>{/if}
    </h2>
    <Chart {option} height={Math.max(320, Math.min(rows.length, 15) * 30 + 100)} />
  </div>

  <div class="panel wide">
    <h2>Répartition mod par mod</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Mod</th>
          <th>Modrinth</th>
          <th>CurseForge</th>
          <th>Total</th>
          <th class="left">Équilibre</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row (row.key)}
          <tr onclick={() => dashboard.openProject(row)}>
            <td class="left name">
              {#if row.icon_url}<img src={row.icon_url} alt="" />{/if}
              {row.title}
            </td>
            <td>{compactNumber(row.modrinth_downloads)}</td>
            <td>{compactNumber(row.curseforge_downloads)}</td>
            <td><b>{compactNumber(total(row))}</b></td>
            <td class="left">
              {#if row.modrinth_id === null || row.curseforge_id === null}
                <span class="solo">
                  {row.curseforge_id === null ? "Modrinth seul" : "CurseForge seul"}
                </span>
              {:else}
                <div class="balance" title="{Math.round(share(row))} % Modrinth">
                  <span class="m" style="width: {share(row)}%"></span>
                  <span class="c" style="width: {100 - share(row)}%"></span>
                </div>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</DetailShell>

<style>
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
    font-family: var(--font-display);
    font-size: 0.98rem;
    font-weight: 600;
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
  tbody tr {
    cursor: pointer;
  }
  tbody tr:hover {
    background: var(--surface-2);
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name img {
    width: 20px;
    height: 20px;
    border-radius: 4px;
  }
  .balance {
    display: flex;
    width: 200px;
    height: 8px;
    border-radius: 4px;
    overflow: hidden;
    background: var(--surface-2);
  }
  .balance .m {
    background: var(--modrinth);
  }
  .balance .c {
    background: var(--curseforge);
  }
  .solo {
    font-size: 0.72rem;
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 6px;
  }
</style>
