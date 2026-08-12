<script lang="ts">
  import type { EventRow } from "../types";

  let { events }: { events: EventRow[] } = $props();
</script>

{#if events.length === 0}
  <p class="empty">Aucun évènement.</p>
{:else}
  <ul>
    {#each events as event (event.occurred_at + event.title)}
      <li>
        <time>{event.occurred_at.slice(0, 10)}</time>
        <span class="kind" title={event.kind}>{event.kind}</span>
        <b title={event.title}>{event.title}</b>
        <span class="detail" title={event.detail}>{event.detail}</span>
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
    overflow: hidden;
    text-overflow: ellipsis;
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
