<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { rankingOption } from "../charts/multiseries";
  import { palette } from "../charts/theme";
  import { api } from "../api";
  import { formatDayLong, formatMonth, formatMoney } from "../format";
  import { dashboard } from "../state.svelte";
  import { theme } from "../theme.svelte";
  import type { AppErrorPayload, CfPointEntry } from "../types";

  /** Résumé en lecture seule : la connexion et la collecte vivent dans les réglages. */
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

  const revenue = $derived(dashboard.overview?.curseforge_revenue ?? null);
  const months = $derived(revenue?.monthly ?? []);
  const latest = $derived(entries.length > 0 ? entries[entries.length - 1] : null);
  const previous = $derived(entries.length > 1 ? entries[entries.length - 2] : null);
  const delta = $derived(latest && previous ? latest.points - previous.points : null);

  /** Les mois vont du plus ancien au plus récent : les barres se lisent de bas
   * en haut, donc l'ordre s'inverse pour retrouver le sens du temps. */
  const chart = $derived(
    rankingOption(
      [...months].reverse().map((m) => formatMonth(m.month)),
      [...months].reverse().map((m) => Number.parseFloat(m.amount_usd) || 0),
      palette(theme.dark),
      palette(theme.dark).curseforge,
    ),
  );
</script>

{#if revenue && revenue.points > 0}
  <div class="value">
    <strong>{formatMoney(revenue.points_usd)}</strong>
    <span class="hint">
      retirables · {revenue.points} points au {latest
        ? formatDayLong(latest.day)
        : "dernier relevé"}
      {#if delta !== null}
        · {delta >= 0 ? "+" : ""}{delta} depuis le relevé précédent
      {/if}
    </span>
  </div>

  <div class="figures">
    {#if revenue.last_month}
      <div class="figure">
        <span class="legend-label">Mois écoulé</span>
        <b>{formatMoney(revenue.last_month)}</b>
      </div>
    {/if}
    {#if revenue.year_to_date}
      <div class="figure">
        <span class="legend-label">Cumul de l'année</span>
        <b>{formatMoney(revenue.year_to_date)}</b>
      </div>
    {/if}
    {#if months.length > 0}
      <div class="figure">
        <span class="legend-label">Mois relevés</span>
        <b>{months.length}</b>
      </div>
    {/if}
  </div>

  {#if months.length > 1}
    <Chart option={chart} height={Math.max(160, months.length * 34 + 30)} />
  {/if}

  <p class="note">
    CurseForge paie en points, à 0,05 $ l'unité, retirables dès qu'ils sont crédités : ce solde
    entre donc dans « retirable maintenant » au même titre que celui de Modrinth. Les montants
    mensuels viennent de ton tableau de bord auteur, relevés à chaque synchronisation.
  </p>

  {#if entries.length > 1}
    <table>
      <thead>
        <tr><th class="left">Relevé du solde</th><th>Points</th><th>Valeur</th></tr>
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
    Aucun relevé pour l'instant. La collecte se fait toute seule à chaque synchronisation ; si ta
    session CurseForge a expiré, les réglages proposent de te reconnecter une fois.
  </p>
{/if}

<style>
  .value {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-bottom: 14px;
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
  .figures {
    display: flex;
    flex-wrap: wrap;
    gap: 22px;
    margin-bottom: 14px;
  }
  .figure {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .figure b {
    font-family: var(--font-mono);
    font-size: 1.05rem;
    font-weight: 600;
  }
  .note {
    margin: 12px 0 0;
    font-size: 0.78rem;
    color: var(--text-dim);
    line-height: 1.5;
    max-width: 80ch;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
    margin-top: 12px;
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
