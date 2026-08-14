<script lang="ts">
  /**
   * Le classement des journées.
   *
   * La page Journée juge une journée à la fois ; celle-ci les met en rang.
   * Deux rangs, plutôt qu'un : celui de la période regardée, qui bouge avec
   * les dates choisies, et celui que la journée avait le jour même, qui ne
   * bougera plus jamais. Les confondre ferait croire qu'un mois faible
   * rétrograde des journées vieilles d'un an.
   */
  import { api } from "../../api";
  import Chart from "../../charts/Chart.svelte";
  import { dailyBarsOption, rankCurveOption } from "../../charts/dayRanking";
  import { palette } from "../../charts/theme";
  import Hint from "../../components/Hint.svelte";
  import RankedTable from "../../components/RankedTable.svelte";
  import { podiumColor } from "../../components/rank";
  import StatRow from "../../components/StatRow.svelte";
  import { compactNumber, formatDay, formatDayLong, formatMoney } from "../../format";
  import { dashboard } from "../../state.svelte";
  import { theme } from "../../theme.svelte";
  import type { AppErrorPayload, DayRankings } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let data = $state<DayRankings | null>(null);
  let loading = $state(false);
  let mode = $state<"jours" | "rang">("jours");
  let order = $state<"rang" | "date">("rang");

  // Les bornes et les plateformes visibles commandent le classement : il se
  // relève dès que l'une d'elles change.
  $effect(() => {
    const [days, from, to, platforms] = [
      dashboard.rangeDays,
      dashboard.rangeFrom,
      dashboard.rangeTo,
      dashboard.visiblePlatforms,
    ];
    loading = true;
    api
      .dayRankings(days, from, to, platforms)
      .then((value) => (data = value))
      .catch((e) => (dashboard.error = (e as AppErrorPayload)?.message ?? String(e)))
      .finally(() => (loading = false));
  });

  const rows = $derived(data?.rows ?? []);
  const measured = $derived(rows.filter((r) => r.total > 0));
  const option = $derived(
    mode === "jours" ? dailyBarsOption(rows, palette(theme.dark)) : rankCurveOption(rows, palette(theme.dark)),
  );

  const best = $derived(measured.find((r) => r.rank_period === 1) ?? null);
  const total = $derived(measured.reduce((sum, r) => sum + r.total, 0));
  const average = $derived(measured.length ? Math.round(total / measured.length) : 0);
  /**
   * Journées qui furent premières le jour de leur passage : ce sont les
   * records successifs, la seule lecture qui distingue une bonne journée d'un
   * sommet.
   */
  const sommets = $derived(measured.filter((r) => r.rank_at_the_time === 1).length);

  const listed = $derived(
    order === "rang"
      ? [...measured].sort((a, b) => (a.rank_period ?? 0) - (b.rank_period ?? 0))
      : [...measured].sort((a, b) => (a.day < b.day ? 1 : -1)),
  );

  const PERIOD =
    "Rang de la journée parmi celles de la période affichée, la meilleure en tête. Changer les dates change ce rang : il ne dit rien d'autre que la place tenue dans ce qui est montré à l'écran.";
  const AT_THE_TIME =
    "Rang de la journée parmi les quatre-vingt-dix qui la précèdent, celle-ci comprise — le rang qu'elle avait le jour où elle s'est produite. Le classement ne regarde jamais en avant, et rien de ce qui est arrivé ensuite ne peut plus le changer. Les journées sans aucun relevé sont écartées, elles flatteraient le rang.";
  const REVENUE =
    "Modrinth relève ses revenus jour par jour. CurseForge n'en publie aucun : ce qui apparaît ici vient de l'écart entre deux soldes de points, relevés au passage seulement, si bien que la plupart des journées n'en portent aucun.";
  const coverage = $derived(
    `Modrinth est relevé depuis le ${data?.first_modrinth_day ? formatDayLong(data.first_modrinth_day) : "—"}, CurseForge depuis le ${data?.first_curseforge_day ? formatDayLong(data.first_curseforge_day) : "—"}. Avant ces dates, un total ne porte que sur l'autre plateforme : il paraît faible sans l'être.`,
  );
</script>

<DetailShell
  title="Classement des journées"
  subtitle="{measured.length} journées relevées{loading ? " · relevé en cours…" : ""}"
>
  {#snippet actions()}
    <div class="switch">
      <button class:active={mode === "jours"} onclick={() => (mode = "jours")}>Par jour</button>
      <button class:active={mode === "rang"} onclick={() => (mode = "rang")}>Rang au fil du temps</button>
    </div>
  {/snippet}

  <StatRow
    stats={[
      { label: "Journées relevées", value: String(measured.length) },
      { label: "Moyenne par jour", value: compactNumber(average) },
      {
        label: "Meilleure journée",
        value: best ? compactNumber(best.total) : "—",
        hint: best ? formatDay(best.day) : "aucun relevé",
      },
      { label: "Journées record", value: String(sommets), hint: "premières le jour même" },
    ]}
  />

  <div class="chart">
    <h2>
      {mode === "jours" ? "Téléchargements par journée" : "Rang au fil du temps"}
      <Hint text={mode === "jours" ? PERIOD : AT_THE_TIME} />
      <Hint text={coverage} />
    </h2>
    <Chart {option} height={420} />
  </div>

  <div class="panel">
    <h2>
      Les journées, une par une
      <Hint text={AT_THE_TIME} />
      <span class="spacer"></span>
      <span class="switch small">
        <button class:active={order === "rang"} onclick={() => (order = "rang")}>Par rang</button>
        <button class:active={order === "date"} onclick={() => (order = "date")}>Par date</button>
      </span>
    </h2>
    {#if loading && data === null}
      <p class="empty">Lecture du classement…</p>
    {:else if listed.length === 0}
      <p class="empty">Aucune journée relevée sur cette période.</p>
    {:else}
      <RankedTable
        ranked={false}
        maxHeight={520}
        columns={[
          { label: "Journée", align: "left" },
          { label: "Rang période" },
          { label: "Rang du jour" },
          { label: "Modrinth" },
          { label: "CurseForge" },
          { label: "Total" },
          { label: "Revenus" },
        ]}
        rows={listed}
        key={(row) => row.day}
      >
        {#snippet cells(row)}
          {@const podium = row.rank_period !== null ? podiumColor(row.rank_period - 1) : null}
          <td class="left">{formatDayLong(row.day)}</td>
          <td>
            <span class="badge" class:podium={podium !== null} style="--rank: {podium ?? ''}">
              {row.rank_period ?? "—"}
            </span>
          </td>
          <td class="dim">
            {row.rank_at_the_time === null ? "—" : `${row.rank_at_the_time} / ${row.compared_days}`}
          </td>
          <td>{compactNumber(row.modrinth)}</td>
          <td>{compactNumber(row.curseforge)}</td>
          <td class="strong">{compactNumber(row.total)}</td>
          <td class="dim">{formatMoney(row.revenue)}</td>
        {/snippet}
      </RankedTable>
      <p class="foot">
        Revenus du jour
        <Hint text={REVENUE} />
      </p>
    {/if}
  </div>
</DetailShell>

<style>
  .switch {
    display: flex;
    gap: 4px;
  }
  .switch button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 7px;
    padding: 5px 12px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .switch.small button {
    padding: 3px 9px;
    font-size: 0.74rem;
  }
  .switch button.active,
  .switch button:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .chart,
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .panel {
    margin-top: 14px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 10px;
    font-size: 0.9rem;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  /* Le rang de la période reprend la pastille des tableaux classés : c'est la
   * même idée, elle doit se lire pareil. */
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    padding: 1px 5px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-dim);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .badge.podium {
    background: color-mix(in srgb, var(--rank) 22%, transparent);
    color: var(--rank);
    font-weight: 600;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 10px 0 0;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .empty {
    margin: 10px 0 0;
    color: var(--text-dim);
    font-size: 0.84rem;
  }
</style>
