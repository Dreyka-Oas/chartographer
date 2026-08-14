<script lang="ts">
  /**
   * Le classement des journées.
   *
   * La page Journée juge une journée à la fois ; celle-ci les met en rang. Le
   * classement se règle plutôt que de s'imposer : sur quoi il porte —
   * téléchargements ou revenus — et à quoi chaque journée se compare — une
   * fenêtre glissante, toute l'histoire, ou la période affichée. Deux filtres
   * plutôt que deux colonnes figées : ils laissent choisir la question, là où
   * deux rangs côte à côte auraient imposé les deux réponses à la fois.
   */
  import { api } from "../../api";
  import Chart from "../../charts/Chart.svelte";
  import { dailyBarsOption, rankCurveOption } from "../../charts/dayRanking";
  import { palette } from "../../charts/theme";
  import Hint from "../../components/Hint.svelte";
  import RankedTable from "../../components/RankedTable.svelte";
  import { podiumColor } from "../../components/rank";
  import Select from "../../components/Select.svelte";
  import StatRow from "../../components/StatRow.svelte";
  import { compactNumber, formatDay, formatDayLong, formatMoney } from "../../format";
  import { dashboard } from "../../state.svelte";
  import { theme } from "../../theme.svelte";
  import type { AppErrorPayload, DayRankings, RankBy } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let data = $state<DayRankings | null>(null);
  let loading = $state(false);
  let mode = $state<"jours" | "rang">("jours");
  let order = $state<"rang" | "date">("rang");

  /** Sur quoi le classement porte. */
  let rankBy = $state<RankBy>("downloads");
  /** À quoi chaque journée se compare, tel que choisi dans le filtre. */
  let windowChoice = $state<"90" | "30" | "all" | "period">("90");

  /** Longueur de la période affichée, en jours, bornes incluses. */
  const periodLength = $derived.by(() => {
    if (dashboard.rangeFrom && dashboard.rangeTo) {
      const span = new Date(dashboard.rangeTo).getTime() - new Date(dashboard.rangeFrom).getTime();
      return Math.round(span / 86_400_000) + 1;
    }
    return dashboard.rangeDays;
  });

  /**
   * Fenêtre transmise à la commande. « La période affichée » n'est pas un cas
   * particulier côté backend : c'est ici qu'elle devient un nombre de jours
   * comme un autre.
   */
  const windowDays = $derived(
    windowChoice === "90"
      ? 90
      : windowChoice === "30"
        ? 30
        : windowChoice === "all"
          ? null
          : periodLength,
  );

  // Les bornes, les plateformes visibles et les deux réglages du classement
  // commandent le résultat : il se relève dès que l'un d'eux change.
  $effect(() => {
    const [days, from, to, platforms, by, window] = [
      dashboard.rangeDays,
      dashboard.rangeFrom,
      dashboard.rangeTo,
      dashboard.visiblePlatforms,
      rankBy,
      windowDays,
    ];
    loading = true;
    api
      .dayRankings(days, from, to, platforms, by, window)
      .then((value) => (data = value))
      .catch((e) => (dashboard.error = (e as AppErrorPayload)?.message ?? String(e)))
      .finally(() => (loading = false));
  });

  const rows = $derived(data?.rows ?? []);
  const measured = $derived(rows.filter((r) => r.total > 0));
  const option = $derived(
    mode === "jours" ? dailyBarsOption(rows, palette(theme.dark)) : rankCurveOption(rows, palette(theme.dark)),
  );

  // La meilleure journée se lit sur les téléchargements bruts, indépendamment
  // du classement affiché : changer de critère ou de fenêtre ne doit pas faire
  // bouger ce qui reste un simple record de téléchargements.
  const best = $derived(
    measured.length ? measured.reduce((top, r) => (r.total > top.total ? r : top)) : null,
  );
  const total = $derived(measured.reduce((sum, r) => sum + r.total, 0));
  const average = $derived(measured.length ? Math.round(total / measured.length) : 0);
  /**
   * Journées premières dans leur propre comparaison : ce que « record » veut
   * dire suit donc le réglage choisi, une fenêtre glissante n'y voyant pas la
   * même chose qu'une comparaison à toute l'histoire.
   */
  const sommets = $derived(measured.filter((r) => r.rank === 1).length);

  const listed = $derived(
    order === "rang"
      ? [...measured].sort((a, b) => (a.rank ?? Infinity) - (b.rank ?? Infinity))
      : [...measured].sort((a, b) => (a.day < b.day ? 1 : -1)),
  );

  const WINDOW_HINT =
    "À quoi chaque journée est comparée pour obtenir son rang. Sur une fenêtre glissante, une journée n'est jugée que sur celles qui la précèdent : le rang qu'elle avait le jour même, et que rien de ce qui est arrivé ensuite ne peut plus changer. Sur toute l'histoire antérieure, un pic ancien pèse sur toutes les journées qui le suivent. Sur la période affichée, le rang répond seulement à « où se situe ce jour dans ce que je regarde », et change avec les dates choisies.";
  const BY_HINT =
    "Ce qui décide du rang. Les téléchargements comptent les deux plateformes visibles. Les revenus, eux, sont pour ainsi dire ceux de Modrinth : CurseForge n'en publie aucun par jour, ils ne sont reconstruits que par l'écart entre deux soldes de points relevés au passage, si bien que la plupart des journées n'en portent aucun.";
  const REVENUE =
    "Modrinth relève ses revenus jour par jour. CurseForge n'en publie aucun : ce qui apparaît ici vient de l'écart entre deux soldes de points, relevés au passage seulement, si bien que la plupart des journées n'en portent aucun.";
  const coverage = $derived(
    `Modrinth est relevé depuis le ${data?.first_modrinth_day ? formatDayLong(data.first_modrinth_day) : "—"}, CurseForge depuis le ${data?.first_curseforge_day ? formatDayLong(data.first_curseforge_day) : "—"}. Avant ces dates, un total ne porte que sur l'autre plateforme : il paraît faible sans l'être.`,
  );
  const rankedOn = $derived(rankBy === "downloads" ? "sur les téléchargements" : "sur les revenus");
</script>

<DetailShell
  title="Classement des journées"
  subtitle="{measured.length} journées relevées{loading ? " · relevé en cours…" : ""}"
>
  {#snippet actions()}
    <div class="filters">
      <Select
        value={rankBy}
        label="Classer sur"
        compact
        onchange={(value) => (rankBy = (value as RankBy) ?? "downloads")}
        options={[
          { value: "downloads", label: "Téléchargements" },
          { value: "revenue", label: "Revenus" },
        ]}
      />
      <Select
        value={windowChoice}
        label="Comparer à"
        compact
        onchange={(value) => (windowChoice = (value as typeof windowChoice) ?? "90")}
        options={[
          { value: "90", label: "les 90 jours précédents" },
          { value: "30", label: "les 30 jours précédents" },
          { value: "all", label: "toute l'histoire antérieure" },
          { value: "period", label: "la période affichée" },
        ]}
      />
    </div>
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
      { label: "Journées record", value: String(sommets), hint: "meilleures dans leur comparaison" },
    ]}
  />

  <div class="chart">
    <h2>
      {mode === "jours" ? "Téléchargements par journée" : "Rang au fil du temps"}
      <Hint text={WINDOW_HINT} />
      <Hint text={coverage} />
    </h2>
    <Chart {option} height={420} />
  </div>

  <div class="panel">
    <h2>
      Les journées classées {rankedOn}
      <Hint text={BY_HINT} />
      <Hint text={WINDOW_HINT} />
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
          { label: "Rang" },
          { label: "Modrinth" },
          { label: "CurseForge" },
          { label: "Total" },
          { label: "Revenus" },
        ]}
        rows={listed}
        key={(row) => row.day}
      >
        {#snippet cells(row)}
          {@const podium = row.rank !== null ? podiumColor(row.rank - 1) : null}
          <td class="left">{formatDayLong(row.day)}</td>
          <td>
            <div class="rankcell">
              <span class="badge" class:podium={podium !== null} style="--rank: {podium ?? ''}">
                {row.rank ?? "—"}
              </span>
              <span class="compared">comparé à {row.compared_days} journée{row.compared_days === 1 ? "" : "s"}</span>
            </div>
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
  .filters {
    display: flex;
    gap: 8px;
  }
  .filters :global(.trigger) {
    min-width: 168px;
  }
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
  /* Le rang de la journée reprend la pastille des tableaux classés : c'est la
   * même idée, elle doit se lire pareil. */
  .rankcell {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }
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
  .compared {
    font-size: 0.66rem;
    color: var(--text-dim);
    white-space: nowrap;
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
