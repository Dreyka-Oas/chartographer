<script lang="ts">
  import { compactNumber } from "../format";
  import type { ProjectSummary } from "../types";
  import Sparkline from "./Sparkline.svelte";

  let {
    projects,
    onselect,
    maxHeight = 0,
  }: {
    projects: ProjectSummary[];
    onselect: (key: string) => void;
    /** Hauteur maximale avant défilement interne. `0` laisse la table s'étendre. */
    maxHeight?: number;
  } = $props();

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

<div class="scroller" style={maxHeight > 0 ? `max-height: ${maxHeight}px` : ""}>
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
              {#if row.icon_url}<img src={row.icon_url} alt="" loading="lazy" />{/if}
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
            <Sparkline values={row.spark} />
          </td>
          <td>{compactNumber(row.modrinth_downloads)}</td>
          <td>{compactNumber(row.curseforge_downloads)}</td>
          <td><b>{compactNumber(row.modrinth_downloads + row.curseforge_downloads)}</b></td>
          <td>{row.followers}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  /*
   * La table défile chez elle quand on lui donne une hauteur : avec plusieurs
   * centaines de projets, la page entière ferait sinon des milliers de pixels.
   */
  .scroller {
    overflow: auto;
    overscroll-behavior: contain;
  }
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
    vertical-align: middle;
    /* L'en-tête reste lisible pendant le défilement interne. */
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--surface);
    box-shadow: inset 0 -1px 0 var(--border);
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
