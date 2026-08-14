<script lang="ts">
  /**
   * Le bilan d'une journée, et une seule.
   *
   * Les autres pages répondent à « combien » ; celle-ci répond à « était-ce un
   * bon jour ». La différence n'est pas de degré : un nombre de téléchargements
   * ne veut rien dire seul, il ne prend sens que rapporté à la veille, aux
   * semaines précédentes et au reste de l'historique. Tout ce qui est montré
   * ici sert ce jugement.
   *
   * La journée d'hier est proposée d'emblée : celle en cours n'est pas finie,
   * et ses chiffres monteraient encore sous les yeux.
   */
  import { api } from "../api";
  import Hint from "../components/Hint.svelte";
  import { compactNumber, deltaPercent, formatDayLong, formatMoney } from "../format";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, DayReport } from "../types";

  const today = new Date();
  const iso = (date: Date) => date.toISOString().slice(0, 10);
  const yesterday = iso(new Date(today.getTime() - 86_400_000));

  let day = $state(yesterday);
  let report = $state<DayReport | null>(null);
  let loading = $state(false);

  function fail(e: unknown) {
    dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
  }

  // Le jour et les plateformes visibles commandent le bilan : il se recharge
  // dès que l'un des deux change.
  $effect(() => {
    const chosen = day;
    const platforms = dashboard.visiblePlatforms;
    loading = true;
    api
      .dayReport(chosen, platforms)
      .then((value) => (report = value))
      .catch(fail)
      .finally(() => (loading = false));
  });

  const downloads = $derived(report?.downloads ?? null);
  const revenue = $derived(report?.revenue ?? null);

  const vsPrevious = $derived(
    downloads ? deltaPercent(downloads.total, downloads.previous) : null,
  );
  const vsAverage = $derived(
    downloads && downloads.average_28 > 0
      ? Math.round(((downloads.total - downloads.average_28) / downloads.average_28) * 100)
      : null,
  );

  /**
   * Le verdict, en un mot.
   *
   * Il repose sur la moyenne des vingt-huit derniers jours plutôt que sur la
   * seule veille : un lundi creux suivi d'un mardi ordinaire donnerait
   * autrement l'illusion d'une belle journée.
   */
  const verdict = $derived.by(() => {
    if (!downloads || downloads.total === 0) return { word: "Aucun relevé", tone: "flat" };
    if (vsAverage === null) return { word: "Pas de repère", tone: "flat" };
    if (vsAverage >= 25) return { word: "Très bon jour", tone: "high" };
    if (vsAverage >= 8) return { word: "Bon jour", tone: "up" };
    if (vsAverage <= -25) return { word: "Jour creux", tone: "low" };
    if (vsAverage <= -8) return { word: "Jour faible", tone: "down" };
    return { word: "Jour ordinaire", tone: "flat" };
  });

  const signed = (value: number) => (value > 0 ? `+${value}` : String(value));
  const step = (days: number) => {
    const next = new Date(`${day}T00:00:00Z`);
    next.setUTCDate(next.getUTCDate() + days);
    const value = iso(next);
    if (value <= iso(today)) day = value;
  };

  const RANK =
    "Rang de la journée parmi les quatre-vingt-dix qui la précèdent, celle-ci comprise. Le classement ne regarde jamais en avant : la question est de savoir si c'était un bon jour quand il s'est produit. Les journées sans aucun relevé sont écartées, elles flatteraient le rang.";
  const AVERAGE =
    "Moyennes des sept et des vingt-huit journées qui précèdent, celle-ci exclue : s'y inclure reviendrait à se comparer à soi-même.";
  const REVENUE =
    "Modrinth relève ses revenus jour par jour. Ceux de CurseForge sont reconstruits par l'écart entre deux soldes de points, la plateforme n'en publiant aucun détail quotidien.";
</script>

<div class="bar">
  <button class="nav" onclick={() => step(-1)} aria-label="Jour précédent">←</button>
  <input type="date" bind:value={day} max={iso(today)} />
  <button class="nav" onclick={() => step(1)} disabled={day >= iso(today)} aria-label="Jour suivant">
    →
  </button>
  <button class="quick" class:on={day === yesterday} onclick={() => (day = yesterday)}>Hier</button>
  <button class="quick" class:on={day === iso(today)} onclick={() => (day = iso(today))}>
    Aujourd'hui
  </button>
  {#if report?.partial}
    <span class="warn">
      Journée en cours, chiffres incomplets
      <Hint
        text="La journée n'est pas finie : ses relevés continueront de monter jusqu'à minuit, et la comparer aux précédentes la désavantage."
      />
    </span>
  {/if}
</div>

{#if loading && report === null}
  <p class="notice">Lecture de la journée…</p>
{:else if report && downloads && revenue}
  <div class="verdict {verdict.tone}">
    <span class="word">{verdict.word}</span>
    <span class="date">{formatDayLong(report.day)}</span>
    {#if report.rank !== null}
      <button class="rank" onclick={() => dashboard.openDetail("days")}>
        {report.rank}<sup>{report.rank === 1 ? "re" : "e"}</sup> journée sur {report.ranked_days}
        <span class="more">voir le classement</span>
      </button>
      <Hint text={RANK} />
    {/if}
  </div>

  <div class="cards">
    <article>
      <span class="label">Téléchargements</span>
      <strong>{compactNumber(downloads.total)}</strong>
      <div class="split">
        <span><i class="dot modrinth"></i>{compactNumber(downloads.modrinth)}</span>
        <span><i class="dot curseforge"></i>{compactNumber(downloads.curseforge)}</span>
      </div>
      <span class="hint">
        {vsPrevious === null ? "pas de veille relevée" : `${signed(vsPrevious)} % vs la veille`}
      </span>
    </article>

    <article>
      <span class="label">Revenus</span>
      <strong>{formatMoney(revenue.total)}</strong>
      <div class="split">
        <span><i class="dot modrinth"></i>{formatMoney(revenue.modrinth)}</span>
        <span><i class="dot curseforge"></i>{formatMoney(revenue.curseforge)}</span>
      </div>
      <span class="hint">
        {formatMoney(revenue.previous)} la veille
        <Hint text={REVENUE} />
      </span>
    </article>

    <article>
      <span class="label">Comparé à l'ordinaire</span>
      <strong class:up={(vsAverage ?? 0) > 0} class:down={(vsAverage ?? 0) < 0}>
        {vsAverage === null ? "—" : `${signed(vsAverage)} %`}
      </strong>
      <div class="split">
        <span>7 j : {compactNumber(Math.round(downloads.average_7))}/j</span>
        <span>28 j : {compactNumber(Math.round(downloads.average_28))}/j</span>
      </div>
      <span class="hint">
        moyenne des jours précédents
        <Hint text={AVERAGE} />
      </span>
    </article>

    <article>
      <span class="label">Abonnés</span>
      <strong class:up={(report.followers_delta ?? 0) > 0} class:down={(report.followers_delta ?? 0) < 0}>
        {report.followers_delta === null ? "—" : signed(report.followers_delta)}
      </strong>
      <div class="split">
        <span class="dim">
          {report.best_day === report.day
            ? "meilleure journée à ce jour"
            : `record : ${compactNumber(report.best_downloads)}`}
        </span>
      </div>
      <span class="hint">
        {report.followers_delta === null
          ? "deux relevés sont nécessaires"
          : "gagnés ou perdus ce jour-là"}
      </span>
    </article>
  </div>

  <div class="panel">
    <span class="head">Ce qui a porté la journée</span>
    {#if report.projects.length === 0}
      <p class="empty">Aucun téléchargement relevé ce jour-là.</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th class="left">Projet</th>
            <th>Modrinth</th>
            <th>CurseForge</th>
            <th>Total</th>
            <th>Veille</th>
            <th>Écart</th>
          </tr>
        </thead>
        <tbody>
          {#each report.projects as project (project.key)}
            {@const gap = project.total - project.previous}
            <tr>
              <td class="left name">
                {#if project.icon_url}<img src={project.icon_url} alt="" loading="lazy" />{/if}
                {project.title}
              </td>
              <td>{compactNumber(project.modrinth)}</td>
              <td>{compactNumber(project.curseforge)}</td>
              <td class="strong">{compactNumber(project.total)}</td>
              <td class="dim">{compactNumber(project.previous)}</td>
              <td class:up={gap > 0} class:down={gap < 0}>{gap === 0 ? "—" : signed(gap)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  {#if report.events.length > 0}
    <div class="panel">
      <span class="head">Ce jour-là</span>
      <ul class="events">
        {#each report.events as event (event.occurred_at + event.title)}
          <li>
            <span class="kind">{event.kind}</span>
            <b>{event.title}</b>
            <span class="dim">{event.detail}</span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
{:else}
  <p class="notice">Aucune donnée pour cette journée.</p>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 16px;
  }
  input,
  button {
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: var(--radius-sm);
    padding: 5px 10px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
    font-variant-numeric: tabular-nums;
  }
  button:hover:not(:disabled),
  input:hover {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .nav {
    padding: 5px 11px;
  }
  .quick {
    color: var(--text-dim);
  }
  .quick.on {
    color: var(--accent);
    border-color: var(--accent);
  }
  .warn {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 0.78rem;
    color: var(--warn);
  }
  /*
   * Le verdict tient la première place : c'est la réponse à la question posée.
   * Le filet de gauche prend la couleur du jugement, sans quoi un mot seul se
   * perdrait en tête de page.
   */
  .verdict {
    display: flex;
    align-items: baseline;
    gap: 14px;
    flex-wrap: wrap;
    padding: 14px 18px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-left: 3px solid var(--rule);
    border-radius: var(--radius);
  }
  .verdict.high {
    border-left-color: var(--modrinth);
  }
  .verdict.up {
    border-left-color: var(--accent);
  }
  .verdict.down {
    border-left-color: var(--warn);
  }
  .verdict.low {
    border-left-color: var(--error);
  }
  .word {
    font-family: var(--font-display);
    font-size: 1.35rem;
    font-weight: 600;
  }
  .date {
    color: var(--text-dim);
    font-size: 0.86rem;
  }
  .rank {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-left: auto;
    font-size: 0.8rem;
    color: var(--text-dim);
  }
  .rank sup {
    font-size: 0.62rem;
  }
  .rank {
    border: 0;
    background: none;
    padding: 0;
  }
  .rank:hover .more {
    color: var(--accent);
  }
  .more {
    font-size: 0.72rem;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .cards {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
    margin-top: 14px;
  }
  @media (min-width: 560px) {
    .cards {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (min-width: 1100px) {
    .cards {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }
  article {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label {
    font-size: 0.75rem;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  strong {
    font-size: 1.7rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  strong.up,
  td.up {
    color: var(--modrinth);
  }
  strong.down,
  td.down {
    color: var(--error);
  }
  .split {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    font-size: 0.76rem;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 2px;
    margin-right: 5px;
    vertical-align: middle;
  }
  .dot.modrinth {
    background: var(--modrinth);
  }
  .dot.curseforge {
    background: var(--curseforge);
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .panel {
    margin-top: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .head {
    font-size: 0.8rem;
    color: var(--text-dim);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 10px;
    font-size: 0.84rem;
  }
  th {
    text-align: right;
    font-weight: 500;
    font-size: 0.74rem;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0 0 8px;
  }
  td {
    text-align: right;
    padding: 7px 0;
    border-top: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .left {
    text-align: left;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .name img {
    width: 22px;
    height: 22px;
    border-radius: 5px;
    object-fit: cover;
  }
  .strong {
    font-weight: 600;
  }
  .dim {
    color: var(--text-dim);
  }
  .events {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 0.82rem;
  }
  .events li {
    display: flex;
    align-items: baseline;
    gap: 9px;
    flex-wrap: wrap;
  }
  .kind {
    font-size: 0.72rem;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-dim);
  }
  .empty,
  .notice {
    margin: 10px 0 0;
    color: var(--text-dim);
    font-size: 0.84rem;
  }
</style>
