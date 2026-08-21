<script lang="ts">
  import { api } from "../api";
  import { formatDayLong, formatMoney, formatRange } from "../format";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, CfPointEntry } from "../types";

  let entries = $state<CfPointEntry[]>([]);
  let loaded = $state(false);
  /** Compte rendu de la dernière collecte, qu'elle vienne d'une synchronisation
   * automatique ou d'une relance manuelle. */
  const report = $derived(dashboard.curseforge);

  function fail(e: unknown) {
    dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
  }

  $effect(() => {
    if (loaded) return;
    loaded = true;
    api
      .curseforgePoints()
      .then((value) => (entries = value))
      .catch(fail);
  });

  const latest = $derived(entries.length > 0 ? entries[entries.length - 1] : null);
  const previous = $derived(entries.length > 1 ? entries[entries.length - 2] : null);
  const delta = $derived(latest && previous ? latest.points - previous.points : null);

  /** Un relevé n'est un problème que s'il a échoué alors qu'aucun solde n'a
   * jamais été enregistré : le reste du temps, l'écran a de quoi s'afficher. */
  const trouble = $derived(
    report?.needs_login === true || (report?.failed === true && latest === null),
  );
</script>

<div class="head">
  <div class="figure">
    <span class="legend-label">Solde CurseForge</span>
    {#if latest}
      <strong>{formatMoney(latest.value_usd)}</strong>
      <span class="hint">
        {latest.points} points · relevé le {formatDayLong(latest.day)}
        {#if delta !== null}
          · {delta >= 0 ? "+" : ""}{delta} depuis la fois précédente
        {/if}
      </span>
    {:else}
      <strong class="void">—</strong>
      <span class="hint">Aucun solde relevé pour l'instant.</span>
    {/if}
  </div>

  <p class="note">
    Ni le programme de points ni l'historique du tableau de bord n'ont d'interface publique, et le
    filtre anti-robot refuse toute requête faite hors d'un navigateur. L'application lit donc le
    tableau de bord depuis une fenêtre invisible, avec ta session : téléchargements quotidiens,
    solde et revenus arrivent à chaque synchronisation. Cette fenêtre ne se montre que si la
    session expire.
  </p>
</div>

{#if trouble}
  <div class="alert">
    <span>
      {#if report?.needs_login}
        Ta session CurseForge a expiré : identifie-toi une fois, la collecte reprend seule ensuite.
      {:else}
        Le dernier relevé n'a rien rapporté. Une reconnexion suffit le plus souvent.
      {/if}
    </span>
    <button class="primary" onclick={() => api.openCurseforgeWindow().catch(fail)}>
      Se reconnecter à CurseForge
    </button>
  </div>
{/if}

{#if report}
  <details>
    <summary>Détail du dernier relevé</summary>
    <div class="detail">
      <span class="source">{report.detail}</span>
      {#if report.visited.length > 0}
        <span class="source">Pages parcourues : {report.visited.join(" · ")}</span>
      {/if}
      {#if report.imported.length > 0}
        <table>
          <thead>
            <tr><th class="left">Mod</th><th>Jours</th><th>Période</th></tr>
          </thead>
          <tbody>
            {#each report.imported as row (row.title)}
              <tr>
                <td class="left">{row.title}</td>
                <td>{row.days}</td>
                <td>{formatRange(row.from, row.to)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </details>
{/if}

{#if entries.length > 1}
  <details>
    <summary>Historique des soldes · {entries.length} relevés</summary>
    <div class="detail">
      <table>
        <thead>
          <tr><th class="left">Relevé</th><th>Points</th><th>Valeur</th></tr>
        </thead>
        <tbody>
          {#each [...entries].reverse().slice(0, 12) as entry (entry.day)}
            <tr>
              <td class="left">{formatDayLong(entry.day)}</td>
              <td>{entry.points}</td>
              <td>{formatMoney(entry.value_usd)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </details>
{/if}

<style>
  /*
   * Le chiffre d'abord, l'explication à côté : la carte n'a plus de bouton
   * esseulé en bas, puisque le relevé se fait tout seul. Ce qui reste, le
   * détail technique, l'historique, se déplie à la demande.
   */
  .head {
    display: flex;
    gap: 24px;
    align-items: flex-start;
    flex-wrap: wrap;
  }
  .figure {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 190px;
  }
  strong {
    font-family: var(--font-mono);
    font-size: 1.6rem;
    font-weight: 600;
    line-height: 1.2;
  }
  .void {
    color: var(--text-dim);
  }
  .hint {
    font-size: 0.76rem;
    color: var(--text-dim);
  }
  .note {
    flex: 1 1 320px;
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-dim);
    line-height: 1.5;
    max-width: 68ch;
  }
  .alert {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 14px;
    padding: 10px 12px;
    border-left: 2px solid var(--warn);
    background: var(--surface-2);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    font-size: 0.82rem;
  }
  .alert span {
    flex: 1 1 260px;
  }
  details {
    margin-top: 12px;
    border-top: 1px solid var(--border);
    padding-top: 10px;
  }
  summary {
    cursor: pointer;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  summary:hover {
    color: var(--text);
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 10px;
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
  button:hover {
    border-color: var(--accent);
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
    font-weight: 600;
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
</style>
