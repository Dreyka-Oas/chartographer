<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { sparklineOption } from "../charts/sparkline";
  import { compactNumber } from "../format";
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
      <th>Tendance</th>
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
          {#if row.icon_url}<img src={row.icon_url} alt="" />{/if}
          <span>{row.title}</span>
          {#if row.link_confidence !== null && row.link_confidence < 1}
            <em title="Appariement automatique incertain">
              lien ~{Math.round(row.link_confidence * 100)} %
            </em>
          {/if}
          {#if row.curseforge_id === null}<em class="solo">Modrinth seul</em>{/if}
          {#if row.modrinth_id === null}<em class="solo">CurseForge seul</em>{/if}
        </td>
        <td class="spark">
          {#if row.spark.length > 1}
            <Chart option={sparklineOption(row.spark)} height={30} />
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
  th {
    text-align: right;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
  }
  th:first-child,
  td:first-child {
    text-align: left;
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
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    text-align: right;
    font-variant-numeric: tabular-nums;
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
    width: 22px;
    height: 22px;
    border-radius: 5px;
  }
  .spark {
    width: 110px;
  }
  em {
    font-style: normal;
    font-size: 0.7rem;
    color: var(--warn);
    border: 1px solid var(--warn);
    border-radius: 4px;
    padding: 1px 5px;
  }
  em.solo {
    color: var(--text-dim);
    border-color: var(--border);
  }
</style>
