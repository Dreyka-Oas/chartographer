<script lang="ts">
  import { api } from "../api";
  import { formatDayLong, formatMoney } from "../format";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, CfPointEntry } from "../types";

  /** Résumé en lecture seule : la saisie et la connexion vivent dans les réglages. */
  let entries = $state<CfPointEntry[]>([]);
  let loaded = $state(false);

  $effect(() => {
    if (loaded) return;
    loaded = true;
    api
      .curseforgePoints()
      .then((value) => (entries = value))
      .catch((e) => (dashboard.error = (e as AppErrorPayload)?.message ?? String(e)));
  });

  const latest = $derived(entries.length > 0 ? entries[entries.length - 1] : null);
  const previous = $derived(entries.length > 1 ? entries[entries.length - 2] : null);
  const delta = $derived(latest && previous ? latest.points - previous.points : null);
</script>

{#if latest}
  <div class="value">
    <strong>{formatMoney(latest.value_usd)}</strong>
    <span class="hint">
      {latest.points} points au {formatDayLong(latest.day)}
      {#if delta !== null}
        · {delta >= 0 ? "+" : ""}{delta} depuis le relevé précédent
      {/if}
    </span>
  </div>

  {#if entries.length > 1}
    <table>
      <thead>
        <tr><th class="left">Relevé</th><th>Points</th><th>Valeur</th></tr>
      </thead>
      <tbody>
        {#each [...entries].reverse().slice(0, 8) as entry (entry.day)}
          <tr>
            <td class="left">{formatDayLong(entry.day)}</td>
            <td>{entry.points}</td>
            <td>{formatMoney(entry.value_usd)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
{:else}
  <p class="empty">
    Aucun relevé enregistré. CurseForge n'expose pas son programme de points : rends-toi dans les
    réglages, section CurseForge, pour te connecter et relever ton solde.
  </p>
{/if}

<style>
  .value {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-bottom: 12px;
  }
  strong {
    font-family: var(--font-mono);
    font-size: 1.45rem;
    font-weight: 600;
  }
  .hint {
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
  }
  th {
    text-align: right;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-weight: 500;
  }
  td {
    text-align: right;
    padding: 5px 8px;
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  tbody tr:last-child td {
    border-bottom: 0;
  }
  .left {
    text-align: left;
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.84rem;
    margin: 0;
    line-height: 1.5;
    max-width: 70ch;
  }
</style>
