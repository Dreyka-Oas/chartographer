<script lang="ts">
  import StatRow from "../../components/StatRow.svelte";
  import type { EventRow } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let { events }: { events: EventRow[] } = $props();

  let kind = $state("");
  let search = $state("");

  const kinds = $derived([...new Set(events.map((e) => e.kind))].sort());
  const rows = $derived(
    events.filter((e) => {
      if (kind && e.kind !== kind) return false;
      if (!search.trim()) return true;
      const needle = search.trim().toLowerCase();
      return (
        e.title.toLowerCase().includes(needle) ||
        e.detail.toLowerCase().includes(needle) ||
        e.kind.toLowerCase().includes(needle)
      );
    }),
  );

  const newest = $derived(events[0]?.occurred_at.slice(0, 10) ?? "—");
  const oldest = $derived(events[events.length - 1]?.occurred_at.slice(0, 10) ?? "—");
</script>

<DetailShell title="Évènements" subtitle="Notifications Modrinth, du plus récent au plus ancien">
  {#snippet actions()}
    <input bind:value={search} placeholder="Rechercher…" />
    <select bind:value={kind}>
      <option value="">Tous les types</option>
      {#each kinds as k (k)}<option value={k}>{k}</option>{/each}
    </select>
  {/snippet}

  <StatRow
    stats={[
      { label: "Évènements", value: String(events.length) },
      { label: "Types distincts", value: String(kinds.length) },
      { label: "Plus récent", value: newest },
      { label: "Plus ancien", value: oldest },
    ]}
  />

  <div class="panel">
    {#if rows.length === 0}
      <p class="empty">Aucun évènement ne correspond au filtre.</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th class="left">Date</th>
            <th class="left">Type</th>
            <th class="left">Projet</th>
            <th class="left">Détail</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as event (event.occurred_at + event.title + event.kind)}
            <tr>
              <td class="left mono">{event.occurred_at.slice(0, 16).replace("T", " ")}</td>
              <td class="left"><span class="kind">{event.kind}</span></td>
              <td class="left">{event.title}</td>
              <td class="left dim">{event.detail}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</DetailShell>

<style>
  input,
  select {
    /* Raccourci `background` évité : il effacerait le chevron dessiné par la
     * feuille globale. */
    background-color: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    padding: 5px 9px;
    font: inherit;
    font-size: 0.8rem;
  }
  select {
    padding-right: 26px;
  }
  input {
    width: 180px;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
  }
  th {
    text-align: left;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-weight: 500;
  }
  td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }
  .left {
    text-align: left;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 0.78rem;
    white-space: nowrap;
  }
  .dim {
    color: var(--text-dim);
    /* Les détails de modération sont du JSON compact : il doit céder, pas la table. */
    overflow-wrap: anywhere;
  }
  .kind {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 6px;
    color: var(--text-dim);
    white-space: nowrap;
  }
  tbody tr:hover {
    background: var(--surface-2);
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
