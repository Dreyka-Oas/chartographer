<script lang="ts">
  import { api } from "../api";
  import { formatDayLong, formatMoney } from "../format";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, CfPointEntry } from "../types";

  let entries = $state<CfPointEntry[]>([]);
  let running = $state(false);
  let loaded = $state(false);
  /** Compte rendu de la dernière collecte, qu'elle vienne d'une synchronisation
   * automatique ou d'une relance manuelle. */
  const report = $derived(dashboard.curseforge);

  function fail(e: unknown) {
    dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
  }

  function refresh() {
    api
      .curseforgePoints()
      .then((value) => (entries = value))
      .catch(fail);
  }

  $effect(() => {
    if (loaded) return;
    loaded = true;
    refresh();
  });

  const latest = $derived(entries.length > 0 ? entries[entries.length - 1] : null);
  const previous = $derived(entries.length > 1 ? entries[entries.length - 2] : null);
  const delta = $derived(latest && previous ? latest.points - previous.points : null);

  /**
   * Une seule action : l'application ouvre le tableau de bord, parcourt les
   * pages et importe ce qu'elle y trouve. La fenêtre ne s'affiche que si la
   * connexion manque.
   */
  async function collect() {
    running = true;
    try {
      await dashboard.collectCurseforge();
      refresh();
      await dashboard.load();
    } catch (e) {
      fail(e);
    } finally {
      running = false;
    }
  }
</script>

<p class="note">
  CurseForge n'expose ni son programme de points ni l'historique de son tableau de bord, et son
  filtre anti-robot refuse toute requête faite hors d'un navigateur. L'application interroge donc
  l'interface du tableau de bord depuis une fenêtre invisible, avec ta session : téléchargements
  quotidiens par mod, solde de points et revenus estimés arrivent à chaque synchronisation, sans
  que tu aies rien à faire. Tu ne verras cette fenêtre que si la session expire.
</p>

<div class="assist">
  <button onclick={collect} disabled={running}>
    {running ? "Collecte en cours…" : "Relever maintenant"}
  </button>
  {#if report?.needs_login}
    <button class="primary" onclick={() => api.openCurseforgeWindow().catch(fail)}>
      Se reconnecter à CurseForge
    </button>
  {/if}
</div>

{#if report}
  <div class="read" class:miss={report.needs_login || report.imported.length === 0}>
    {#if report.needs_login}
      <span>
        Connexion nécessaire : la fenêtre CurseForge vient de s'ouvrir. Identifie-toi, puis relance
        la collecte. La session est ensuite conservée.
      </span>
    {:else}
      {#if report.points !== null}
        <span>Solde relevé : <b>{report.points} points</b>.</span>
      {/if}
      {#if report.imported.length > 0}
        <span class="legend-label">Historiques importés</span>
        <table>
          <thead>
            <tr><th class="left">Mod</th><th>Jours</th><th>Période</th></tr>
          </thead>
          <tbody>
            {#each report.imported as row (row.title)}
              <tr>
                <td class="left">{row.title}</td>
                <td>{row.days}</td>
                <td>{row.from} → {row.to}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <span>
          Aucune série n'a pu être rattachée à un de tes mods. Le détail ci-dessous dit ce qui a été
          parcouru ; envoie-le-moi si le compte n'y est pas.
        </span>
      {/if}
    {/if}
    <span class="source">{report.detail}</span>
    {#if report.visited.length > 0}
      <span class="source">Pages parcourues : {report.visited.join(" · ")}</span>
    {/if}
  </div>
{/if}

{#if latest}
  <div class="summary">
    <span class="legend-label">Dernier solde relevé</span>
    <strong>{formatMoney(latest.value_usd)}</strong>
    <span class="hint">
      {latest.points} points · {formatDayLong(latest.day)}
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
        {#each [...entries].reverse().slice(0, 10) as entry (entry.day)}
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
  <p class="empty">Aucun solde relevé pour l'instant.</p>
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
    margin-bottom: 14px;
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
  }
  .source {
    font-size: 0.74rem;
    color: var(--text-dim);
    overflow-wrap: anywhere;
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
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
    font-weight: 600;
  }
  .summary {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-bottom: 12px;
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
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
