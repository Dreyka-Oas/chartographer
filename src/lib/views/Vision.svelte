<script lang="ts">
  import Card from "../components/Card.svelte";
  import EventsFeed from "../components/EventsFeed.svelte";
  import FreshnessBadge from "../components/FreshnessBadge.svelte";
  import Hint from "../components/Hint.svelte";
  import KpiBand from "../components/KpiBand.svelte";
  import LoaderHeatmap from "../components/LoaderHeatmap.svelte";
  import PlatformSplit from "../components/PlatformSplit.svelte";
  import ProjectsTable from "../components/ProjectsTable.svelte";
  import RangePicker from "../components/RangePicker.svelte";
  import Timeline from "../components/Timeline.svelte";
  import WorldMap from "../components/WorldMap.svelte";
  import { dashboard } from "../state.svelte";

  const overview = $derived(dashboard.overview);

  /**
   * Pays réellement situés sur la carte. Le relevé porte une ligne `??` pour les
   * téléchargements dont Modrinth ne connaît pas l'origine : la compter parmi
   * les pays annonçait un de plus que ce que la carte montre, et la note du bas
   * dit déjà ce qu'il en est.
   */
  const mappedCountries = $derived(
    (overview?.countries ?? []).filter((entry) => entry.country !== "??"),
  );
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

  <!--
    Chaque carte porte sa propre bascule : elle ne change pas la fenêtre lue,
    seulement la lecture de cette carte-là.
  -->
  <KpiBand
    kpis={overview.kpis}
    bind:ranged={dashboard.kpiRanged}
    days={overview.days.length}
    onfollowers={() => dashboard.openDetail("followers")}
  />

  {#if !dashboard.platforms.modrinth || !dashboard.platforms.curseforge}
    <p class="notice">
      {dashboard.platforms.modrinth ? "CurseForge" : "Modrinth"} est masqué.
      <Hint
        text={dashboard.platforms.modrinth
          ? "Ses téléchargements ne sont pas comptés dans les chiffres affichés. Clique la pastille en haut de la fenêtre pour la réafficher."
          : "Ses téléchargements ne sont pas comptés ici. L'origine géographique, les versions de jeu et les revenus ne sont relevés que sur Modrinth : leurs cartes sont donc retirées. Clique la pastille en haut de la fenêtre pour la réafficher."}
      />
    </p>
  {/if}

  <!--
    Un seul jour de snapshots CurseForge : le dire vaut mieux que de laisser
    croire à une courbe plate. Le pourquoi, lui, tient dans l'infobulle, il ne
    se lit qu'une fois.
  -->
  {#if dashboard.platforms.curseforge && overview.curseforge_history_days < 2}
    <p class="notice">
      {overview.curseforge_history_days === 0
        ? "L'historique CurseForge est encore vide : la courbe se construira au fil des relevés."
        : "L'historique CurseForge ne compte qu'une seule journée pour l'instant."}
      <Hint
        text="CurseForge n'expose aucun historique public : Chartographer va le chercher sur ton tableau de bord auteur, à chaque synchronisation, et reconstruit les journées en comparant deux relevés. Si ce chiffre ne monte pas, ta session CurseForge a sans doute expiré : les réglages proposent de te reconnecter."
      />
    </p>
  {/if}

  <div class="grid">
    <Card
      title="Téléchargements par jour"
      subtitle="Relevés des tableaux de bord ; CurseForge complété par écart de snapshots"
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
        subtitle="{mappedCountries.length} pays relevés"
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
  /*
   * Bloc d'état, calé à droite. Il garde la même ligne que les filtres tant
   * qu'il y tient ; passé à la ligne, il occupe toute la largeur plutôt que de
   * flotter en tête d'une rangée vide.
   */
  .state {
    margin-left: auto;
    flex: 1 1 auto;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 10px 12px;
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
     * Une colonne par défaut, deux dès qu'un graphique y reste lisible. Jamais
     * trois : au-delà les courbes deviennent illisibles et la dernière rangée
     * laisse d'autant plus de trous.
     */
    /* `minmax(0, …)` et non `1fr` seul : le minimum implicite d'une piste est
     * la largeur de son contenu, et un graphique la poussait au-delà de la
     * fenêtre, la carte débordait sur une fenêtre étroite. */
    grid-template-columns: minmax(0, 1fr);
    /*
     * Hauteur commune et bornée : le contenu s'étire pour la remplir, et une
     * liste plus longue défile chez elle au lieu d'étirer toute la rangée.
     */
    grid-auto-rows: 420px;
    gap: 16px;
    margin-top: 16px;
  }
  @media (min-width: 1100px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    /*
     * Nombre impair de cartes : la dernière prend les deux colonnes au lieu de
     * laisser un blanc à côté d'elle. Le compte change selon les plateformes
     * masquées, la règle vaut donc pour tous les cas sans être recalculée.
     */
    .grid > :global(section:last-child:nth-child(odd)) {
      grid-column: 1 / -1;
    }
  }
  .full {
    margin-top: 16px;
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 14px 0 0;
    padding: 10px 14px;
    border-left: 2px solid var(--warn);
    background: var(--surface-2);
    border-radius: 0 var(--radius) var(--radius) 0;
    color: var(--text-dim);
    font-size: 0.82rem;
  }
</style>
