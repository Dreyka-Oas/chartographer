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
    /**
     * Hauteur au-delà de laquelle le tableau défile chez lui, en pixels. `0` le
     * laisse s'étendre. Passé quelques dizaines de lignes, une liste complète
     * repousse tout ce qui la suit hors de l'écran sans rien apprendre de plus.
     */
    maxHeight = 0,
    /** Rend les lignes cliquables ; le curseur le dit. */
    onselect,
    empty = "Rien à classer pour l'instant.",
  }: {
    columns: Column[];
    rows: Row[];
    key: (row: Row, index: number) => string;
    cells: Snippet<[Row, number]>;
    ranked?: boolean;
    maxHeight?: number;
    onselect?: (row: Row, index: number) => void;
    /**
     * Ce que le tableau dit quand il n'a aucune ligne. Sans cela, il posait ses
     * en-têtes de colonnes au-dessus du vide.
     */
    empty?: string;
  } = $props();
</script>

{#if rows.length === 0}
  <p class="empty">{empty}</p>
{:else}
<div
  class="scroller"
  class:capped={maxHeight > 0}
  style={maxHeight > 0 ? `max-height: ${maxHeight}px` : ""}
>
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
</div>
{/if}

<style>
  .empty {
    color: var(--text-dim);
    font-size: 0.86rem;
    margin: 0;
    padding: 8px 0;
  }
  .scroller {
    /* La gouttière est réservée en permanence : sans elle, l'apparition de la
     * barre décalerait les colonnes d'une dizaine de pixels. */
    overflow: auto;
    scrollbar-gutter: stable;
    overscroll-behavior: contain;
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
  /* Tableau plafonné : l'en-tête reste lisible pendant le défilement. Le filet
   * passe en ombre intérieure, une bordure de cellule ne suivant pas un
   * en-tête collé. */
  .capped th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--surface);
    border-bottom: 0;
    box-shadow: inset 0 -1px 0 var(--border);
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
