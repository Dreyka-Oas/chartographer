<script lang="ts">
  /**
   * Les abonnés : combien ils sont, d'où ils viennent, comment leur nombre
   * bouge.
   *
   * Seul CurseForge nomme qui suit un compte, sur sa fiche publique. Modrinth
   * n'en donne que le compte : ses adresses ont été sondées une à une, token en
   * main, `project/{id}/followers`, `user/{id}/followers`,
   * `analytics/followers` et leurs variantes rendent toutes 404, c'est-à-dire
   * une route qui n'existe pas, et non une porte fermée. La seule qui réponde,
   * `user/{id}/follows`, rend les projets que l'on suit soi-même.
   *
   * Les noms relevés ne sont pas affichés : ce qui compte ici, c'est le nombre
   * et son mouvement. Ils servent à autre chose, en comparant la liste d'un
   * jour à celle de la veille, l'application sait qui vient d'arriver et qui
   * est parti, quand aucune des deux plateformes ne date un abonnement.
   */
  import { api } from "../../api";
  import Chart from "../../charts/Chart.svelte";
  import { followersOption } from "../../charts/followers";
  import { palette } from "../../charts/theme";
  import Hint from "../../components/Hint.svelte";
  import StatRow from "../../components/StatRow.svelte";
  import Tooltip from "../../components/Tooltip.svelte";
  import { formatDayLong } from "../../format";
  import { dashboard } from "../../state.svelte";
  import { theme } from "../../theme.svelte";
  import type { AppErrorPayload, FollowersReport } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let report = $state<FollowersReport | null>(null);
  let loading = $state(true);
  let collecting = $state(false);
  let note = $state("");

  function fail(e: unknown) {
    dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
  }

  // Ce qui est déjà connu s'affiche tout de suite. Le relevé, lui, se fait avec
  // la collecte quotidienne : y retourner à l'ouverture de cette page ferait un
  // passage de plus sur une fiche publique qui n'a rien demandé.
  $effect(() => {
    if (report !== null) return;
    api
      .curseforgeFollowers()
      .then((value) => (report = value))
      .catch(fail)
      .finally(() => (loading = false));
  });

  async function refresh() {
    collecting = true;
    note = "";
    try {
      const fresh = await api.collectCurseforgeFollowers();
      report = fresh;
      note = fresh.detail;
    } catch (e) {
      fail(e);
    } finally {
      collecting = false;
    }
  }

  const history = $derived(report?.history ?? []);
  const option = $derived(followersOption(history, palette(theme.dark)));
  const followers = $derived(report?.followers ?? []);

  /**
   * D'où viennent les abonnés, au dernier relevé. La courbe dira l'évolution,
   * mais elle demande plusieurs jours : cette répartition-là se lit dès le
   * premier passage.
   */
  const latest = $derived(history[history.length - 1] ?? null);
  const total = $derived((latest?.modrinth ?? 0) + (latest?.curseforge ?? 0));
  const modrinthShare = $derived(total > 0 ? ((latest?.modrinth ?? 0) / total) * 100 : 50);
  const present = $derived(followers.filter((f) => f.lost_on === null));
  const gone = $derived(followers.filter((f) => f.lost_on !== null));
  const dated = $derived(present.filter((f) => f.arrival_known));
  /** Les mouvements que l'application a réellement vus passer. */
  const moves = $derived(followers.filter((f) => f.arrival_known || f.lost_on !== null));

  const SOURCES =
    "Modrinth compte ses abonnés projet par projet, et l'application en fait la somme. CurseForge n'en donne qu'un total, lu en tête de la fiche publique du compte.";
  const CURVE =
    "Ni Modrinth ni CurseForge ne tiennent d'historique d'abonnés : ils n'annoncent que le compte du jour. La courbe se construit donc ici, un relevé après l'autre, et ne remontera jamais avant le premier d'entre eux. Les barres donnent l'écart avec la veille, calculé de la même façon.";
  const NAMES =
    "CurseForge est le seul des deux à nommer qui suit un compte. Les noms relevés ne sont pas affichés : ils servent à repérer les arrivées et les départs, en comparant la liste d'un jour à celle de la veille.";
</script>

<DetailShell title="Abonnés" subtitle="Modrinth et CurseForge, relevés une fois par jour">
  {#snippet actions()}
    <button onclick={refresh} disabled={collecting}>
      {collecting ? "Relevé en cours…" : "Relever maintenant"}
    </button>
  {/snippet}

  <StatRow
    stats={[
      { label: "Abonnés", value: String(total) },
      { label: "Modrinth", value: String(latest?.modrinth ?? 0) },
      { label: "CurseForge", value: String(latest?.curseforge ?? 0) },
      { label: "Arrivées datées", value: String(dated.length) },
    ]}
  />

  {#if note}<p class="note">{note}</p>{/if}

  <div class="panel chart">
    <span class="head">
      D'où viennent tes abonnés
      <Hint text={SOURCES} />
    </span>
    {#if latest === null}
      <p class="empty">Aucun relevé pour l'instant.</p>
    {:else}
      <!--
        La répartition du dernier relevé, lisible dès le premier jour. La barre
        donne la proportion d'un coup d'œil, les deux mentions le compte exact.
      -->
      <Tooltip block text="{latest.modrinth} Modrinth · {latest.curseforge} CurseForge">
        <div class="bar">
          <span class="modrinth" style="width:{modrinthShare}%"></span>
          <span class="curseforge"></span>
        </div>
      </Tooltip>
      <div class="split">
        <span><i class="dot modrinth"></i>{latest.modrinth} Modrinth</span>
        <span><i class="dot curseforge"></i>{latest.curseforge} CurseForge</span>
        <span class="dim">relevé le {formatDayLong(latest.day)}</span>
      </div>
    {/if}

    <span class="head second">
      Évolution
      <Hint text={CURVE} />
    </span>
    {#if history.length === 0}
      <p class="empty">La courbe se remplira au premier relevé.</p>
    {:else}
      <!--
        Un seul point se trace aussi : c'est un relevé, pas une courbe, mais il
        vaut mieux le montrer que d'annoncer qu'il n'y a rien à voir.
      -->
      <Chart {option} height={260} />
    {/if}
  </div>

  {#if !loading && (moves.length > 0 || gone.length > 0)}
    <div class="panel">
      <span class="head">
        Mouvements constatés
        <Hint text={NAMES} />
      </span>
      <div class="moves">
        {#each moves as follower (follower.name)}
          <span class="move" class:out={follower.lost_on !== null}>
            {follower.lost_on
              ? `Un abonné perdu le ${formatDayLong(follower.lost_on)}`
              : `Un abonné gagné le ${formatDayLong(follower.first_seen)}`}
          </span>
        {/each}
      </div>
    </div>
  {/if}
</DetailShell>

<style>
  button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: var(--radius-sm);
    padding: 5px 12px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .note {
    margin: 12px 0 0;
    font-size: 0.8rem;
    color: var(--text-dim);
  }
  .panel {
    margin-top: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .chart {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 0.8rem;
    color: var(--text-dim);
  }
  .head.second {
    margin-top: 10px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  /* Deux segments accolés : la part Modrinth donne la largeur, CurseForge
   * occupe ce qui reste. C'est un seul tout partagé, sans écart entre eux. */
  .bar {
    display: flex;
    height: 6px;
    border-radius: 999px;
    overflow: hidden;
    background: var(--surface-2);
  }
  .bar .modrinth {
    background: var(--modrinth);
  }
  .bar .curseforge {
    background: var(--curseforge);
    flex: 1;
  }
  .split {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    font-size: 0.78rem;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .dot {
    display: inline-block;
    width: 7px;
    height: 7px;
    border-radius: 2px;
    margin-right: 6px;
    vertical-align: middle;
  }
  .dot.modrinth {
    background: var(--modrinth);
  }
  .dot.curseforge {
    background: var(--curseforge);
  }
  .moves {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }
  .move {
    font-size: 0.76rem;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--modrinth);
  }
  .move.out {
    color: var(--error);
  }
  .dim {
    color: var(--text-dim);
  }
  .empty {
    margin: 0;
    max-width: 60ch;
    color: var(--text-dim);
    font-size: 0.84rem;
    line-height: 1.55;
  }
</style>
