<script lang="ts">
  import { api } from "../api";
  import { formatDayLong, formatMoney } from "../format";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, CfPointEntry } from "../types";

  let entries = $state<CfPointEntry[]>([]);
  let draft = $state("");
  let saving = $state(false);
  let loaded = $state(false);

  function report(e: unknown) {
    dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
  }

  function refresh() {
    api
      .curseforgePoints()
      .then((value) => (entries = value))
      .catch(report);
  }

  $effect(() => {
    if (loaded) return;
    loaded = true;
    refresh();
  });

  const latest = $derived(entries.length > 0 ? entries[entries.length - 1] : null);
  const previous = $derived(entries.length > 1 ? entries[entries.length - 2] : null);
  const delta = $derived(latest && previous ? latest.points - previous.points : null);

  async function save() {
    const points = Number.parseInt(draft, 10);
    if (!Number.isFinite(points) || points < 0) return;
    saving = true;
    try {
      await api.recordCurseforgePoints(points);
      draft = "";
      refresh();
    } catch (e) {
      report(e);
    } finally {
      saving = false;
    }
  }

  async function forget(day: string) {
    try {
      await api.forgetCurseforgePoints(day);
      refresh();
    } catch (e) {
      report(e);
    }
  }
</script>

<p class="note">
  CurseForge rémunère ses auteurs en points, sans aucune interface pour les lire : ni l'API publique,
  ni le jeton de dépôt n'y donnent accès. Relève ton solde sur ton tableau de bord auteur et note-le
  ici ; l'application le convertit au tarif annoncé par CurseForge, 0,05 $ le point, et suit son
  évolution.
</p>

<form
  onsubmit={(e) => {
    e.preventDefault();
    save();
  }}
>
  <label>
    <span class="legend-label">Solde du jour</span>
    <input type="number" min="0" step="1" bind:value={draft} placeholder="points" />
  </label>
  <button type="submit" disabled={saving || draft === ""}>
    {saving ? "Enregistrement…" : "Enregistrer"}
  </button>
</form>

{#if latest}
  <div class="summary">
    <div>
      <span class="legend-label">Dernier relevé</span>
      <strong>{formatMoney(latest.value_usd)}</strong>
      <span class="hint">
        {latest.points} points · {formatDayLong(latest.day)}
        {#if delta !== null}
          · {delta >= 0 ? "+" : ""}{delta} depuis le relevé précédent
        {/if}
      </span>
    </div>
  </div>

  <table>
    <thead>
      <tr><th class="left">Relevé</th><th>Points</th><th>Valeur</th><th></th></tr>
    </thead>
    <tbody>
      {#each [...entries].reverse() as entry (entry.day)}
        <tr>
          <td class="left">{formatDayLong(entry.day)}</td>
          <td>{entry.points}</td>
          <td>{formatMoney(entry.value_usd)}</td>
          <td class="right">
            <button class="ghost" onclick={() => forget(entry.day)} title="Supprimer ce relevé">
              ✕
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p class="empty">Aucun relevé enregistré pour l'instant.</p>
{/if}

<style>
  .note {
    margin: 0 0 14px;
    font-size: 0.8rem;
    color: var(--text-dim);
    line-height: 1.5;
    max-width: 80ch;
  }
  form {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    margin-bottom: 14px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  input {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 7px 10px;
    font: inherit;
    font-size: 0.86rem;
    font-variant-numeric: tabular-nums;
    width: 130px;
  }
  button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: var(--radius-sm);
    padding: 7px 13px;
    font: inherit;
    font-size: 0.84rem;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .ghost {
    border-color: transparent;
    background: none;
    color: var(--text-dim);
    padding: 2px 7px;
  }
  .ghost:hover {
    color: var(--error);
    border-color: var(--error);
  }
  .summary {
    display: flex;
    gap: 20px;
    margin-bottom: 14px;
  }
  .summary div {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  strong {
    font-family: var(--font-mono);
    font-size: 1.35rem;
    font-weight: 600;
  }
  .hint {
    font-size: 0.76rem;
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
  .right {
    width: 1%;
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
