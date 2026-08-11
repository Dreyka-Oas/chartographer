<script lang="ts">
  import Card from "../components/Card.svelte";
  import EventsFeed from "../components/EventsFeed.svelte";
  import FreshnessBadge from "../components/FreshnessBadge.svelte";
  import KpiBand from "../components/KpiBand.svelte";
  import LoaderHeatmap from "../components/LoaderHeatmap.svelte";
  import PlatformSplit from "../components/PlatformSplit.svelte";
  import ProjectsTable from "../components/ProjectsTable.svelte";
  import RangePicker from "../components/RangePicker.svelte";
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

  {#if !dashboard.platforms.modrinth || !dashboard.platforms.curseforge}
    <p class="notice">
      {dashboard.platforms.modrinth ? "CurseForge" : "Modrinth"} est masqué : ses téléchargements ne
      sont pas comptés ici.
      {#if !dashboard.platforms.modrinth}
        L'origine géographique, les versions de jeu et les revenus ne sont relevés que sur Modrinth,
        leurs cartes sont donc retirées.
      {/if}
      Clique la pastille en haut de la fenêtre pour la réafficher.
    </p>
  {/if}

  {#if dashboard.platforms.curseforge && overview.curseforge_history_days < 2}
    <p class="notice">
      CurseForge n'expose aucun historique public : Chartographer va le chercher sur ton tableau de
      bord auteur, à chaque synchronisation. {overview.curseforge_history_days} jour est enregistré
      pour l'instant. Si ce chiffre ne monte pas, ta session CurseForge a sans doute expiré : les
      réglages proposent de te reconnecter une fois.
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

    <!--
      Origine, versions et revenus ne sont relevés que sur Modrinth : masquer
      cette plateforme retire les cartes plutôt que de les laisser vides.
    -->
    {#if dashboard.platforms.modrinth}
      <Card
        title="Origine des téléchargements"
        subtitle="{overview.countries.length} pays relevés"
        onexpand={() => dashboard.openDetail("countries")}
      >
        <WorldMap countries={overview.countries} />
      </Card>
    {/if}

    <!-- Comparer deux plateformes suppose que les deux soient affichées. -->
    {#if dashboard.platforms.modrinth && dashboard.platforms.curseforge}
      <Card
        title="Modrinth contre CurseForge"
        subtitle="Total par projet, trié par volume"
        onexpand={() => dashboard.openDetail("platforms")}
      >
        <PlatformSplit projects={overview.per_project} />
      </Card>
    {/if}

    <Card
      title="Versions de jeu et loaders"
      subtitle="Répartition des publications des deux plateformes"
      onexpand={() => dashboard.openDetail("loaders")}
    >
      <LoaderHeatmap cells={overview.loaders} />
    </Card>

    <!-- Tout ce qui touche à l'argent vit dans l'onglet Revenus. -->

    <Card
      title="Évènements"
      subtitle="{overview.events.length} notifications"
      onexpand={() => dashboard.openDetail("events")}
    >
      <EventsFeed events={overview.events} />
    </Card>
  </div>

  <!--
    Le tableau vit hors de la grille : sa hauteur suit le nombre de projets,
    alors que les cartes de la grille partagent une hauteur commune.
  -->
  <div class="full">
    <Card
      title="Tous les projets"
      subtitle="Clique une ligne pour le détail du mod"
      onexpand={() => dashboard.openDetail("projects")}
    >
      <ProjectsTable
        projects={overview.per_project}
        maxHeight={520}
        onselect={(key) => {
          const found = overview.per_project.find((p) => p.key === key);
          if (found) dashboard.openProject(found);
        }}
      />
    </Card>
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
    grid-template-columns: repeat(auto-fit, minmax(520px, 1fr));
    /*
     * Hauteur commune et bornée : le contenu s'étire pour la remplir, et une
     * liste plus longue défile chez elle au lieu d'étirer toute la rangée.
     */
    grid-auto-rows: 420px;
    gap: 16px;
    margin-top: 16px;
  }
  .full {
    margin-top: 16px;
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
