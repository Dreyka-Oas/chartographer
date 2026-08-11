<script lang="ts">
  import { api } from "../api";
  import { formatDayLong, formatMoney } from "../format";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, CfAnalysis, CfPointEntry, PairingEntry } from "../types";

  let entries = $state<CfPointEntry[]>([]);
  let projects = $state<PairingEntry[]>([]);
  let draft = $state("");
  let pasted = $state("");
  let analysis = $state<CfAnalysis | null>(null);
  let target = $state<number | null>(null);
  let saving = $state(false);
  let importing = $state(false);
  let imported = $state("");
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
    api
      .pairingState()
      .then((value) => (projects = value.filter((p) => p.platform === "curseforge")))
      .catch(report);
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

  async function analyse() {
    if (!pasted.trim()) return;
    imported = "";
    try {
      analysis = await api.analyzeCurseforgeText(pasted);
      if (analysis.points !== null && draft === "") draft = String(analysis.points);
    } catch (e) {
      report(e);
    }
  }

  async function importSeries() {
    if (target === null) return;
    importing = true;
    try {
      const days = await api.importCurseforgeSeries(target, pasted);
      imported = `${days} jours enregistrés.`;
      await dashboard.load();
    } catch (e) {
      report(e);
    } finally {
      importing = false;
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
  CurseForge n'expose ni son programme de points ni l'historique de son tableau de bord : ni l'API
  publique, ni le jeton de dépôt n'y donnent accès, et le site refuse de s'afficher dans une fenêtre
  intégrée. Ouvre-le dans ton navigateur, où tu es déjà connecté, puis rapporte ici ce que tu y vois.
</p>

<div class="assist">
  <button onclick={() => api.openCurseforgeSite().catch(report)}>
    Ouvrir mon tableau de bord CurseForge
  </button>
</div>

<label class="paste">
  <span class="legend-label">Contenu rapporté</span>
  <textarea
    bind:value={pasted}
    rows="4"
    placeholder="Colle ici le texte de la page (pour le solde de points) ou la réponse JSON d'une requête de statistiques (onglet Réseau du navigateur)"
  ></textarea>
</label>
<div class="assist">
  <button onclick={analyse} disabled={!pasted.trim()}>Analyser</button>
</div>

{#if analysis}
  <div class="read" class:miss={analysis.points === null && analysis.days === 0}>
    {#if analysis.points !== null}
      <span>Solde reconnu : <b>{analysis.points} points</b>.</span>
    {/if}
    {#if analysis.days > 0}
      <span>
        Historique reconnu : <b>{analysis.days} jours</b> du {analysis.from} au {analysis.to}, pour
        {analysis.total.toLocaleString("fr-FR")} au total.
      </span>
      <div class="import">
        <select bind:value={target}>
          <option value={null}>À quel mod appartient cet historique…</option>
          {#each projects as project (project.id)}
            <option value={project.id}>{project.title}</option>
          {/each}
        </select>
        <button onclick={importSeries} disabled={target === null || importing}>
          {importing ? "Import…" : "Importer cet historique"}
        </button>
      </div>
      {#if imported}<span class="ok">{imported}</span>{/if}
    {/if}
    {#if analysis.points === null && analysis.days === 0}
      <span>
        Rien de reconnaissable : ni solde de points, ni série datée. Pour l'historique, ouvre
        l'onglet Réseau du navigateur, recharge la page des statistiques, et copie la réponse d'une
        requête qui renvoie du JSON.
      </span>
    {/if}
    {#if analysis.excerpt}<span class="source">« {analysis.excerpt} »</span>{/if}
  </div>
{/if}

<form
  onsubmit={(e) => {
    e.preventDefault();
    save();
  }}
>
  <label>
    <span class="legend-label">Solde de points du jour</span>
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
  <p class="empty">Aucun relevé de points enregistré pour l'instant.</p>
{/if}

<style>
  .note {
    margin: 0 0 14px;
    font-size: 0.8rem;
    color: var(--text-dim);
    line-height: 1.5;
    max-width: 82ch;
  }
  .assist {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .paste {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }
  textarea {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 8px 10px;
    font: inherit;
    font-size: 0.8rem;
    font-family: var(--font-mono);
    resize: vertical;
    width: 100%;
  }
  .read {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0 0 14px;
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
  .import {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 4px;
  }
  .ok {
    color: var(--modrinth);
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
  input,
  select {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 7px 10px;
    font: inherit;
    font-size: 0.86rem;
    font-variant-numeric: tabular-nums;
  }
  input {
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
