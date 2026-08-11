<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { sparklineOption } from "../charts/sparkline";
  import { palette } from "../charts/theme";
  import { compactNumber } from "../format";
  import { theme } from "../theme.svelte";
  import type { ProjectSummary } from "../types";

  let { projects, onselect }: { projects: ProjectSummary[]; onselect: (key: string) => void } =
    $props();

  type Column = "title" | "total" | "modrinth" | "curseforge" | "followers";
  let sortBy = $state<Column>("total");
  let ascending = $state(false);

  const total = (p: ProjectSummary) => p.modrinth_downloads + p.curseforge_downloads;

  const rows = $derived(
    [...projects].sort((a, b) => {
      const direction = ascending ? 1 : -1;
      switch (sortBy) {
        case "title":
          return a.title.localeCompare(b.title) * direction;
        case "modrinth":
          return (a.modrinth_downloads - b.modrinth_downloads) * direction;
        case "curseforge":
          return (a.curseforge_downloads - b.curseforge_downloads) * direction;
        case "followers":
          return (a.followers - b.followers) * direction;
        default:
          return (total(a) - total(b)) * direction;
      }
    }),
  );

  function sort(column: Column) {
    if (sortBy === column) {
      ascending = !ascending;
    } else {
      sortBy = column;
      ascending = false;
    }
  }
</script>

<table>
  <thead>
    <tr>
      <th><button onclick={() => sort("title")}>Projet</button></th>
      <th class="spark">Tendance</th>
      <th><button onclick={() => sort("modrinth")}>Modrinth</button></th>
      <th><button onclick={() => sort("curseforge")}>CurseForge</button></th>
      <th><button onclick={() => sort("total")}>Total</button></th>
      <th><button onclick={() => sort("followers")}>Followers</button></th>
    </tr>
  </thead>
  <tbody>
    {#each rows as row (row.key)}
      <tr onclick={() => onselect(row.key)}>
        <td class="name">
          <span class="cell">
            {#if row.icon_url}<img src={row.icon_url} alt="" />{/if}
            <span class="label">{row.title}</span>
            {#if row.link_confidence !== null && row.link_confidence < 1}
              <em title="Appariement automatique incertain">
                lien ~{Math.round(row.link_confidence * 100)} %
              </em>
            {/if}
            {#if row.curseforge_id === null}<em class="solo">Modrinth seul</em>{/if}
            {#if row.modrinth_id === null}<em class="solo">CurseForge seul</em>{/if}
          </span>
        </td>
        <td class="spark">
          {#if row.spark.length > 1}
            <Chart option={sparklineOption(row.spark, palette(theme.dark))} height={32} />
          {/if}
        </td>
        <td>{compactNumber(row.modrinth_downloads)}</td>
        <td>{compactNumber(row.curseforge_downloads)}</td>
        <td><b>{compactNumber(row.modrinth_downloads + row.curseforge_downloads)}</b></td>
        <td>{row.followers}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  /*
   * Colonnes de texte et de chiffres réduites à leur contenu : c'est la colonne
   * de tendance qui absorbe toute la largeur restante, au lieu de laisser un
   * long vide entre le nom du mod et sa courbe.
   */
  th,
  td {
    width: 1%;
    white-space: nowrap;
  }
  th {
    text-align: right;
    padding: 7px 10px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }
  th:first-child,
  td:first-child {
    text-align: left;
    padding-left: 0;
  }
  th:last-child,
  td:last-child {
    padding-right: 0;
  }
  th button {
    background: none;
    border: 0;
    color: var(--text-dim);
    font: inherit;
    cursor: pointer;
    padding: 0;
  }
  th button:hover {
    color: var(--text);
  }
  td {
    padding: 7px 10px;
    border-bottom: 1px solid var(--border);
    text-align: right;
    vertical-align: middle;
    font-variant-numeric: tabular-nums;
  }
  tbody tr:last-child td {
    border-bottom: 0;
  }
  tbody tr {
    cursor: pointer;
  }
  tbody tr:hover {
    background: var(--surface-2);
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cell img {
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spark {
    /* Seule colonne élastique : elle prend tout l'espace disponible. */
    width: auto;
    min-width: 120px;
    text-align: center;
  }
  td.spark {
    padding: 4px 10px;
  }
  em {
    font-style: normal;
    font-size: 0.7rem;
    color: var(--warn);
    border: 1px solid var(--warn);
    border-radius: 6px;
    padding: 1px 6px;
    flex-shrink: 0;
  }
  em.solo {
    color: var(--text-dim);
    border-color: var(--border);
  }
</style>
