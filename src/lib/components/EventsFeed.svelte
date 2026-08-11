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
        <span class="kind">{event.kind}</span>
        <b>{event.title}</b>
        <span class="detail">{event.detail}</span>
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
    /* La liste bute net : plus de rebond qui déborde sur la page derrière. */
    overscroll-behavior: contain;
  }
  li {
    display: grid;
    grid-template-columns: 84px 110px 1fr;
    gap: 8px;
    align-items: baseline;
    font-size: 0.82rem;
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
    grid-column: 1 / -1;
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
