<script lang="ts" generics="Row">
  /**
   * Tableau classé : un rang par ligne, les trois premiers en couleur, et le
   * survol qui suit la ligne.
   *
   * Les tableaux de l'application se ressemblaient au point d'avoir chacun sa
   * copie des mêmes règles d'alignement, de filets et de survol. Elles vivent
   * ici une seule fois ; l'appelant ne décrit plus que ses colonnes et ce qu'il
   * met dedans.
   *
   * ```svelte
   * <RankedTable columns={[{ label: "Pays", align: "left" }, { label: "Part" }]}
   *              rows={pays} key={(p) => p.code} onselect={ouvrir}>
   *   {#snippet cells(row)}
   *     <td class="left">{row.nom}</td>
   *     <td>{row.part}</td>
   *   {/snippet}
   * </RankedTable>
   * ```
   *
   * L'appelant écrit ses `<td>` lui-même : les cellules portent des barres, des
   * pastilles ou du texte selon les tableaux, et les enfermer dans une
   * description figée aurait coûté plus qu'elle n'aurait fait gagner.
   */
  import type { Snippet } from "svelte";
  import { podiumColor, type Column } from "./rank";

  let {
    columns,
    rows,
    key,
    cells,
    /** Colonne de rang. La couper laisse le tableau, ses filets et son survol. */
    ranked = true,
    /** Rend les lignes cliquables ; le curseur le dit. */
    onselect,
  }: {
    columns: Column[];
    rows: Row[];
    key: (row: Row, index: number) => string;
    cells: Snippet<[Row, number]>;
    ranked?: boolean;
    onselect?: (row: Row, index: number) => void;
  } = $props();
</script>

<table class:clickable={Boolean(onselect)}>
  <thead>
    <tr>
      {#if ranked}
        <th class="rank-head" aria-label="Rang"></th>
      {/if}
      {#each columns as column (column.label)}
        <th class:left={column.align === "left"}>{column.label}</th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each rows as row, i (key(row, i))}
      <tr
        onclick={onselect ? () => onselect(row, i) : undefined}
        onkeydown={onselect
          ? (event) => {
              // Une ligne cliquable doit s'ouvrir au clavier aussi.
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onselect(row, i);
              }
            }
          : undefined}
        tabindex={onselect ? 0 : undefined}
        role={onselect ? "button" : undefined}
      >
        {#if ranked}
          {@const color = podiumColor(i)}
          <td class="rank">
            <span class="badge" class:podium={color !== null} style="--rank: {color ?? ''}">
              {i + 1}
            </span>
          </td>
        {/if}
        {@render cells(row, i)}
      </tr>
    {/each}
  </tbody>
</table>

<style>
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
  th.left {
    text-align: left;
  }
  .rank-head {
    width: 30px;
  }
  /* Les cellules viennent de l'appelant : le style les rejoint par `:global`,
   * faute de quoi Svelte le retirerait comme inutilisé. */
  table :global(td) {
    text-align: right;
    padding: 5px 8px;
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  table :global(td.left) {
    text-align: left;
  }
  table :global(td.dim) {
    color: var(--text-dim);
  }
  tbody tr:hover {
    background: var(--surface-2);
  }
  tbody tr:focus-visible {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
  }
  .clickable tbody tr {
    cursor: pointer;
  }
  td.rank {
    padding-right: 0;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-dim);
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
  }
  /* Le rang est déjà donné par l'ordre des lignes : la couleur le redit, elle
   * ne le porte pas seule. */
  .badge.podium {
    background: color-mix(in srgb, var(--rank) 22%, transparent);
    color: var(--rank);
    font-weight: 600;
  }
</style>
