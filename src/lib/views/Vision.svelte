<script lang="ts">
  import Card from "../components/Card.svelte";
  import EventsFeed from "../components/EventsFeed.svelte";
  import FreshnessBadge from "../components/FreshnessBadge.svelte";
  import KpiBand from "../components/KpiBand.svelte";
  import LoaderHeatmap from "../components/LoaderHeatmap.svelte";
  import PlatformSplit from "../components/PlatformSplit.svelte";
  import ProjectsTable from "../components/ProjectsTable.svelte";
  import RangePicker from "../components/RangePicker.svelte";
  import RevenueChart from "../components/RevenueChart.svelte";
  import Timeline from "../components/Timeline.svelte";
  import WorldMap from "../components/WorldMap.svelte";
  import { dashboard } from "../state.svelte";

  const overview = $derived(dashboard.overview);
</script>

{#if overview}
  <div class="toolbar">
    <RangePicker />
    <div class="state">
      <FreshnessBadge entries={overview.freshness} />
      <button class="sync" onclick={() => dashboard.sync()} disabled={dashboard.syncing}>
        {dashboard.syncing ? "Synchronisation…" : "Synchroniser"}
      </button>
    </div>
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
      onexpand={() => dashboard.openDetail("timeline")}
    >
      <Timeline points={overview.timeline} />
    </Card>

    <Card
      title="Origine des téléchargements"
      subtitle="{overview.countries.length} pays relevés"
      onexpand={() => dashboard.openDetail("countries")}
    >
      <WorldMap countries={overview.countries} />
    </Card>

    <Card
      title="Modrinth contre CurseForge"
      subtitle="Total par projet, trié par volume"
      onexpand={() => dashboard.openDetail("platforms")}
    >
      <PlatformSplit projects={overview.per_project} />
    </Card>

    <Card
      title="Versions de jeu et loaders"
      subtitle="Concentration des téléchargements Modrinth"
      onexpand={() => dashboard.openDetail("loaders")}
    >
      <LoaderHeatmap cells={overview.loaders} />
    </Card>

    <Card
      title="Revenus"
      subtitle="Journalier, cumulé et échéancier de reversement"
      onexpand={() => dashboard.openDetail("revenue")}
    >
      <RevenueChart points={overview.revenue} />
    </Card>

    <Card
      title="Évènements"
      subtitle="{overview.events.length} notifications"
      onexpand={() => dashboard.openDetail("events")}
    >
      <EventsFeed events={overview.events} />
    </Card>

    <div class="wide">
      <Card
        title="Tous les projets"
        subtitle="Clique une ligne pour le détail du mod"
        onexpand={() => dashboard.openDetail("projects")}
      >
        <ProjectsTable
          projects={overview.per_project}
          onselect={(key) => {
            const found = overview.per_project.find((p) => p.key === key);
            if (found) dashboard.openProject(found);
          }}
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
    gap: 14px 20px;
    flex-wrap: wrap;
    margin-bottom: 16px;
  }
  .state {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  button {
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: var(--radius-sm);
    padding: 5px 12px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
    transition:
      color 120ms ease,
      border-color 120ms ease;
  }
  button:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .sync {
    color: var(--text);
  }
  .sync:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .grid {
    display: grid;
    /*
     * Deux colonnes larges plutôt que quatre étroites : au-delà, les graphiques
     * deviennent illisibles et la dernière rangée reste à moitié vide.
     */
    grid-template-columns: repeat(auto-fit, minmax(560px, 1fr));
    /*
     * Rangées de hauteur régulière : les cartes d'une même ligne se répondent,
     * et leur contenu s'étire pour occuper la place au lieu de laisser du vide.
     */
    grid-auto-rows: minmax(340px, auto);
    gap: 18px;
    margin-top: 18px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  .notice {
    margin: 14px 0 0;
    padding: 10px 14px;
    border-left: 2px solid var(--warn);
    background: var(--surface-2);
    border-radius: 0 var(--radius) var(--radius) 0;
    color: var(--text-dim);
    font-size: 0.82rem;
  }
</style>
