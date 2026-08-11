<script lang="ts">
  import Chart from "../../charts/Chart.svelte";
  import { rankingOption } from "../../charts/multiseries";
  import { scheduleOption } from "../../charts/payout";
  import { revenueOption } from "../../charts/revenue";
  import { palette } from "../../charts/theme";
  import StatRow from "../../components/StatRow.svelte";
  import { formatMoney } from "../../format";
  import { theme } from "../../theme.svelte";
  import type { Overview } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let { overview }: { overview: Overview } = $props();

  const payout = $derived(overview.payout);
  const money = (raw: string) => formatMoney(raw || "0");
  const num = (raw: string) => Number.parseFloat(raw) || 0;

  const periodTotal = $derived(overview.revenue.reduce((s, r) => s + num(r.amount), 0));
  const future = $derived(payout.schedule.filter((x) => x.future));
  const futureTotal = $derived(future.reduce((s, x) => s + num(x.amount), 0));
  const nextDue = $derived(future[0]);

  const daily = $derived(revenueOption(overview.revenue, palette(theme.dark)));
  const schedule = $derived(scheduleOption(payout.schedule, palette(theme.dark)));

  const byProject = $derived([...overview.revenue_by_project].slice(0, 12));
  const projectChart = $derived(
    rankingOption(
      [...byProject].reverse().map((r) => r.title),
      [...byProject].reverse().map((r) => num(r.amount)),
      palette(theme.dark),
    ),
  );
  const projectTotal = $derived(overview.revenue_by_project.reduce((s, r) => s + num(r.amount), 0));
</script>

<DetailShell
  title="Revenus"
  subtitle="Programme de reversement Modrinth · montants en dollars"
>
  <StatRow
    stats={[
      {
        label: "Retirable maintenant",
        value: money(payout.available),
        hint: "solde disponible sur Modrinth",
      },
      {
        label: "En attente de maturation",
        value: money(payout.pending),
        hint: nextDue ? `prochaine échéance le ${nextDue.date.slice(0, 10)}` : undefined,
      },
      {
        label: "Revenus à venir",
        value: money(String(futureTotal)),
        hint: `${future.length} échéance(s) programmée(s)`,
      },
      {
        label: "Déjà retiré",
        value: money(payout.withdrawn_lifetime),
        hint: `dont ${money(payout.withdrawn_ytd)} cette année`,
      },
    ]}
  />

  <div class="panel">
    <h2>Échéancier de reversement</h2>
    <p class="note">
      Chaque barre est une échéance mensuelle. Les barres claires sont déjà mûres, les barres
      orangées correspondent aux montants qui se débloqueront après aujourd'hui.
    </p>
    {#if payout.schedule.length === 0}
      <p class="empty">Aucun échéancier reçu. Lance une synchronisation.</p>
    {:else}
      <Chart option={schedule} height={330} />
    {/if}
  </div>

  <div class="grid">
    <div class="panel">
      <h2>Revenus quotidiens sur la période</h2>
      <p class="note">Total de la fenêtre : {money(String(periodTotal))}.</p>
      {#if overview.revenue.length === 0}
        <p class="empty">Aucun revenu enregistré sur cette période.</p>
      {:else}
        <Chart option={daily} height={320} />
      {/if}
    </div>

    <div class="panel">
      <h2>Répartition par mod</h2>
      {#if byProject.length === 0}
        <p class="empty">Aucun revenu attribué à un mod sur cette période.</p>
      {:else}
        <Chart option={projectChart} height={Math.max(220, byProject.length * 30 + 40)} />
      {/if}
    </div>
  </div>

  <div class="panel wide">
    <h2>Détail par mod</h2>
    <table>
      <thead>
        <tr><th class="left">Mod</th><th>Revenus</th><th>Part</th></tr>
      </thead>
      <tbody>
        {#each overview.revenue_by_project as row (row.key)}
          <tr>
            <td class="left">{row.title}</td>
            <td>{money(row.amount)}</td>
            <td>{projectTotal ? ((num(row.amount) / projectTotal) * 100).toFixed(1) : "0"} %</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <p class="disclaimer">
    CurseForge n'apparaît pas ici : son programme de rémunération n'expose aucune API publique de
    revenus, ni pour l'auteur ni pour un tiers. Seuls les téléchargements y sont lisibles.
  </p>
</DetailShell>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
    gap: 14px;
    margin-top: 14px;
  }
  .wide {
    margin-top: 14px;
    max-width: 680px;
  }
  h2 {
    margin: 0 0 8px;
    font-family: var(--font-display);
    font-size: 0.98rem;
    font-weight: 600;
  }
  .note {
    margin: 0 0 10px;
    font-size: 0.78rem;
    color: var(--text-dim);
    max-width: 80ch;
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
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
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .left {
    text-align: left;
  }
  .disclaimer {
    margin: 16px 0 0;
    padding: 10px 14px;
    border-left: 2px solid var(--warn);
    background: var(--surface-2);
    border-radius: 0 var(--radius) var(--radius) 0;
    color: var(--text-dim);
    font-size: 0.8rem;
    max-width: 90ch;
  }
</style>
