<script lang="ts">
  import Select from "../../components/Select.svelte";
  import { formatDayLong } from "../../format";
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

  /**
   * Les horodatages arrivent en ISO. Ils sont écrits comme les dates du reste
   * de l'application — `18 août 2026`, et non `2026-08-18` — l'heure gardant sa
   * forme chiffrée, seule chose qui compte pour situer deux évènements du même
   * jour l'un par rapport à l'autre.
   */
  const dayOf = (iso: string) => formatDayLong(iso.slice(0, 10));
  const stamp = (iso: string) => `${dayOf(iso)} à ${iso.slice(11, 16)}`;

  const filtered = $derived(kind !== "" || search.trim() !== "");
  const shownKinds = $derived([...new Set(rows.map((e) => e.kind))].sort());
  const newest = $derived(rows[0] ? dayOf(rows[0].occurred_at) : "—");
  const oldest = $derived(rows.length > 0 ? dayOf(rows[rows.length - 1].occurred_at) : "—");
</script>

<!-- Sous-titre court : la barre porte aussi un champ de recherche et une liste,
     et une phrase plus longue se repliait sur deux lignes, ce qui faisait
     grandir l'en-tête de cette vue seule. Le tri se lit dans les dates. -->
<DetailShell title="Évènements" subtitle="Notifications Modrinth">
  {#snippet actions()}
    <input bind:value={search} placeholder="Rechercher…" />
    <div class="list">
      <Select
        bind:value={kind}
        label="Type d'évènement"
        align="end"
        compact
        options={[
          { value: "", label: "Tous les types" },
          ...kinds.map((k) => ({ value: k, label: k })),
        ]}
      />
    </div>
  {/snippet}

  <!-- Les compteurs portent sur ce que le tableau montre. Restés sur le total,
       ils annonçaient quatre évènements au-dessus d'une seule ligne dès qu'un
       filtre était posé ; le total est alors rappelé en dessous. -->
  <StatRow
    stats={[
      {
        label: "Évènements",
        value: String(rows.length),
        hint: filtered ? `sur ${events.length} au total` : undefined,
      },
      { label: "Types distincts", value: String(shownKinds.length) },
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
              <td class="left when">{stamp(event.occurred_at)}</td>
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
  input {
    background-color: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    padding: 5px 9px;
    font: inherit;
    font-size: 0.8rem;
  }
  /* Largeur fixe : les types sont de longueurs très inégales. */
  .list {
    width: 170px;
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
  /* La date reste sur une ligne : repliée, elle ferait grandir la rangée sans
   * rien gagner, la colonne ayant la place. */
  .when {
    font-size: 0.82rem;
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
