<script lang="ts">
  import type { EventRow } from "../types";
  import Tooltip from "./Tooltip.svelte";

  let { events }: { events: EventRow[] } = $props();
</script>

{#if events.length === 0}
  <p class="empty">Aucun évènement.</p>
{:else}
  <ul>
    {#each events as event (event.occurred_at + event.title)}
      <li>
        <time>{event.occurred_at.slice(0, 10)}</time>
        <!--
          La bulle enveloppe le texte, pas la cellule : c'est la cellule qui
          tient sa colonne dans la grille, et le texte tronqué à l'intérieur.
        -->
        <span class="kind">
          <Tooltip block text={event.kind}><span class="cut">{event.kind}</span></Tooltip>
        </span>
        <b>
          <Tooltip block text={event.title}><span class="cut">{event.title}</span></Tooltip>
        </b>
        <span class="detail">
          <Tooltip block text={event.detail}><span class="cut">{event.detail}</span></Tooltip>
        </span>
      </li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-gutter: stable;
    /* La liste bute net : plus de rebond qui déborde sur la page derrière. */
    overscroll-behavior: contain;
  }
  /*
   * Colonnes contraintes et texte tronqué : les détails bruts des évènements
   * de modération sont du JSON, qui débordait sur la colonne voisine.
   */
  li {
    display: grid;
    grid-template-columns: 78px minmax(0, 128px) minmax(0, 1fr);
    gap: 4px 8px;
    align-items: baseline;
    font-size: 0.82rem;
  }
  li > * {
    min-width: 0;
    overflow: hidden;
  }
  .cut {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  time {
    white-space: nowrap;
  }
  time,
  .kind,
  .detail {
    color: var(--text-dim);
  }
  .kind {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .detail {
    grid-column: 2 / -1;
    font-size: 0.78rem;
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
