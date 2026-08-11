<script lang="ts">
  import { api } from "../api";
  import { formatDayLong, formatMoney } from "../format";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, CfPointEntry, CfScrape } from "../types";

  let entries = $state<CfPointEntry[]>([]);
  let draft = $state("");
  let saving = $state(false);
  let loaded = $state(false);
  let reading = $state(false);
  let scrape = $state<CfScrape | null>(null);

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

  /**
   * Lecture assistée : l'utilisateur se connecte dans la fenêtre CurseForge,
   * l'application relit ce que la page affiche et propose la valeur trouvée.
   * Rien n'est enregistré sans sa validation.
   */
  async function readFromPage() {
    reading = true;
    scrape = null;
    try {
      scrape = await api.readCurseforgePage();
      if (scrape.points !== null) draft = String(scrape.points);
    } catch (e) {
      report(e);
    } finally {
      reading = false;
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
  ni le jeton de dépôt n'y donnent accès. Deux façons de renseigner ton solde : l'ouvrir dans une
  fenêtre et laisser l'application relire la page, ou le recopier à la main. Il est converti au tarif
  annoncé par CurseForge, 0,05 $ le point.
</p>

<div class="assist">
  <button onclick={() => api.openCurseforgeWindow().catch(report)}>
    Ouvrir CurseForge et se connecter
  </button>
  <button onclick={readFromPage} disabled={reading}>
    {reading ? "Lecture…" : "Lire le solde affiché"}
  </button>
</div>

{#if scrape}
  <p class="read" class:miss={scrape.points === null}>
    {#if scrape.points === null}
      Aucun solde reconnu sur cette page. Ouvre la page qui affiche tes points, puis relance la
      lecture — ou saisis le montant à la main.
    {:else}
      Solde trouvé : <b>{scrape.points} points</b>. Vérifie puis enregistre.
    {/if}
    <span class="source">Page lue : {scrape.title || scrape.url}</span>
    {#if scrape.excerpt}<span class="source">« {scrape.excerpt} »</span>{/if}
  </p>
{/if}

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
  .assist {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .read {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0 0 12px;
    padding: 10px 12px;
    border-left: 2px solid var(--modrinth);
    background: var(--surface-2);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    font-size: 0.82rem;
  }
  .read.miss {
    border-left-color: var(--warn);
    color: var(--text-dim);
  }
  .source {
    font-size: 0.74rem;
    color: var(--text-dim);
    overflow-wrap: anywhere;
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
