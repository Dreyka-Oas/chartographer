<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { rankingOption } from "../charts/multiseries";
  import { scheduleOption } from "../charts/payout";
  import { revenueOption } from "../charts/revenue";
  import { palette } from "../charts/theme";
  import Card from "../components/Card.svelte";
  import CurseforgeSummary from "../components/CurseforgeSummary.svelte";
  import RangePicker from "../components/RangePicker.svelte";
  import StatRow from "../components/StatRow.svelte";
  import { formatMoney, formatPercent } from "../format";
  import { dashboard } from "../state.svelte";
  import { theme } from "../theme.svelte";

  const overview = $derived(dashboard.overview);
  const payout = $derived(overview?.payout ?? null);
  const money = (raw: string) => formatMoney(raw || "0");
  const num = (raw: string) => Number.parseFloat(raw) || 0;

  const periodTotal = $derived((overview?.revenue ?? []).reduce((s, r) => s + num(r.amount), 0));
  const future = $derived((payout?.schedule ?? []).filter((x) => x.future));
  const futureTotal = $derived(future.reduce((s, x) => s + num(x.amount), 0));
  const nextDue = $derived(future[0]);

  const daily = $derived(revenueOption(overview?.revenue ?? [], palette(theme.dark)));
  const schedule = $derived(scheduleOption(payout?.schedule ?? [], palette(theme.dark)));

  /** Au-delà, les barres deviennent illisibles : le tableau prend le relais. */
  const byProject = $derived((overview?.revenue_by_project ?? []).slice(0, 12));
  const projectChart = $derived(
    rankingOption(
      [...byProject].reverse().map((r) => r.title),
      [...byProject].reverse().map((r) => num(r.amount)),
      palette(theme.dark),
    ),
  );
  const projectTotal = $derived(
    (overview?.revenue_by_project ?? []).reduce((s, r) => s + num(r.amount), 0),
  );
</script>

{#if !overview || !payout}
  <p class="notice">Aucune donnée. Lance une synchronisation.</p>
{:else if !dashboard.platforms.modrinth}
  <p class="notice">
    Modrinth est masqué. Le programme de reversement est propre à cette plateforme : réaffiche-la
    depuis la pastille en haut de la fenêtre pour retrouver tes revenus. La carte CurseForge, elle,
    reste disponible ci-dessous.
  </p>
{:else}
  <div class="toolbar">
    <RangePicker />
  </div>

  <StatRow
    stats={[
      {
        label: "Gagné depuis l'origine",
        value: money(overview.kpis.revenue_total),
        hint: `Modrinth ${money(overview.kpis.revenue_modrinth)} · CurseForge ${money(
          overview.kpis.revenue_curseforge,
        )}`,
      },
      {
        label: "Retirable maintenant",
        value: money(overview.kpis.revenue_available),
        hint: `Modrinth ${money(payout.available)} · CurseForge ${money(
          overview.kpis.revenue_curseforge,
        )}, retirables sans attente`,
      },
      {
        label: "En maturation",
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

  <div class="grid">
    <div class="wide">
      <Card
        title="Échéancier de reversement"
        subtitle="Les barres orangées se débloqueront après aujourd'hui"
      >
        {#if payout.schedule.length === 0}
          <p class="empty">Aucun échéancier reçu. Lance une synchronisation.</p>
        {:else}
          <Chart option={schedule} height={330} />
        {/if}
      </Card>
    </div>

    <Card
      title="Revenus quotidiens"
      subtitle="Total de la fenêtre : {money(String(periodTotal))}"
    >
      {#if overview.revenue.length === 0}
        <p class="empty">Aucun revenu enregistré sur cette période.</p>
      {:else}
        <Chart option={daily} height={320} />
      {/if}
      <p class="note">
        Les relevés quotidiens de Modrinth ne remontent pas jusqu'aux débuts d'un projet : ce total
        reste inférieur au cumul, qui vient du solde de reversement.
      </p>
    </Card>

    <Card
      title="Répartition par mod"
      subtitle={overview.revenue_by_project.length > byProject.length
        ? `${byProject.length} premiers · ${overview.revenue_by_project.length - byProject.length} autres dans le tableau`
        : "Sur la fenêtre affichée"}
    >
      {#if byProject.length === 0}
        <p class="empty">Aucun revenu attribué à un mod sur cette période.</p>
      {:else}
        <Chart option={projectChart} height={Math.max(220, byProject.length * 30 + 40)} />
      {/if}
    </Card>

    <div class="wide">
      <Card title="Détail par mod" subtitle="Part de chaque mod sur la fenêtre">
        <!-- Sans revenu attribué, la carte le dit plutôt que de poser un
             en-tête de tableau au-dessus de rien : sa voisine, nourrie de la
             même donnée, l'annonçait déjà. -->
        {#if overview.revenue_by_project.length === 0}
          <p class="empty">Aucun revenu attribué à un mod sur cette période.</p>
        {:else}
          <table>
            <thead>
              <tr><th class="left">Mod</th><th>Revenus</th><th>Part</th></tr>
            </thead>
            <tbody>
              {#each overview.revenue_by_project as row (row.key)}
                <tr>
                  <td class="left">{row.title}</td>
                  <td>{money(row.amount)}</td>
                  <td>{formatPercent(num(row.amount), projectTotal)} %</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </Card>
    </div>
  </div>

{/if}

<!--
  La carte CurseForge vit hors des conditions ci-dessus : c'est justement quand
  Modrinth est masqué ou la base vide qu'on vient y chercher son solde. Elle ne
  disparaît que si l'on masque CurseForge lui-même.
-->
{#if dashboard.platforms.curseforge}
  <div class="grid tail">
    <div class="wide">
      <Card
        title="CurseForge — points et revenus"
        subtitle="Relevé automatiquement sur ton tableau de bord auteur, à chaque synchronisation"
      >
        <CurseforgeSummary />
      </Card>
    </div>
  </div>
{/if}

<style>
  .toolbar {
    margin-bottom: 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(460px, 1fr));
    gap: 16px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  .tail {
    margin-top: 16px;
  }
  .note {
    margin: 10px 0 0;
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
  tbody tr:last-child td {
    border-bottom: 0;
  }
  .left {
    text-align: left;
  }
  .notice {
    margin: 16px 0 0;
    padding: 10px 14px;
    border-left: 2px solid var(--warn);
    background: var(--surface-2);
    border-radius: 0 var(--radius) var(--radius) 0;
    color: var(--text-dim);
    font-size: 0.82rem;
    max-width: 90ch;
  }
  .notice {
    margin-top: 0;
  }
</style>
