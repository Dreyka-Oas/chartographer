<script lang="ts">
  import Card from "../components/Card.svelte";
  import EventsFeed from "../components/EventsFeed.svelte";
  import FreshnessBadge from "../components/FreshnessBadge.svelte";
  import KpiBand from "../components/KpiBand.svelte";
  import LoaderHeatmap from "../components/LoaderHeatmap.svelte";
  import PlatformSplit from "../components/PlatformSplit.svelte";
  import ProjectsTable from "../components/ProjectsTable.svelte";
  import RevenueChart from "../components/RevenueChart.svelte";
  import Timeline from "../components/Timeline.svelte";
  import WorldMap from "../components/WorldMap.svelte";
  import { dashboard } from "../state.svelte";

  const RANGES = [30, 90, 180, 365];
  const overview = $derived(dashboard.overview);
</script>

{#if overview}
  <div class="toolbar">
    <div class="ranges">
      {#each RANGES as days (days)}
        <button
          class:active={dashboard.rangeDays === days}
          onclick={() => dashboard.setRange(days)}
        >
          {days} j
        </button>
      {/each}
    </div>
    <FreshnessBadge entries={overview.freshness} />
    <button class="sync" onclick={() => dashboard.sync()} disabled={dashboard.syncing}>
      {dashboard.syncing ? "Synchronisation…" : "Synchroniser"}
    </button>
  </div>

  <KpiBand kpis={overview.kpis} />

  {#if overview.curseforge_history_days < 2}
    <p class="notice">
      L'historique CurseForge se construit par snapshots quotidiens :
      {overview.curseforge_history_days} jour(s) enregistré(s). La courbe CurseForge restera plate
      jusqu'au deuxième snapshot.
    </p>
  {/if}

  <div class="grid">
    <Card
      title="Téléchargements par jour"
      subtitle="Modrinth en série, CurseForge reconstruit par snapshots"
    >
      <Timeline points={overview.timeline} />
    </Card>

    <Card title="Origine des téléchargements">
      <WorldMap countries={overview.countries} />
    </Card>

    <Card title="Modrinth contre CurseForge" subtitle="Total par projet, trié par volume">
      <PlatformSplit projects={overview.per_project} />
    </Card>

    <Card title="Versions de jeu et loaders" subtitle="Concentration des téléchargements Modrinth">
      <LoaderHeatmap cells={overview.loaders} />
    </Card>

    <Card title="Revenus" subtitle="Journalier et cumulé">
      <RevenueChart points={overview.revenue} />
    </Card>

    <Card title="Évènements">
      <EventsFeed events={overview.events} />
    </Card>

    <div class="wide">
      <Card title="Tous les projets" subtitle="Clique une ligne pour le détail">
        <ProjectsTable
          projects={overview.per_project}
          onselect={(key) => (dashboard.selectedProject = key)}
        />
      </Card>
    </div>
  </div>
{:else if dashboard.loading}
  <p class="notice">Chargement…</p>
{:else}
  <p class="notice">Aucune donnée. Lance une synchronisation.</p>
{/if}

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    margin-bottom: 14px;
  }
  .ranges {
    display: flex;
    gap: 4px;
  }
  button {
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 7px;
    padding: 5px 12px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  button.active,
  button:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .sync {
    margin-left: auto;
    color: var(--text);
  }
  .sync:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(440px, 1fr));
    gap: 14px;
    margin-top: 14px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  .notice {
    margin: 14px 0 0;
    padding: 10px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 0.83rem;
  }
</style>
