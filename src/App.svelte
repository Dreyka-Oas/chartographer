<script lang="ts">
  import { cubicOut } from "svelte/easing";
  import { boot } from "./lib/boot.svelte";
  import PlatformBadge from "./lib/components/PlatformBadge.svelte";
  import ThemeToggle from "./lib/components/ThemeToggle.svelte";
  import { dashboard } from "./lib/state.svelte";
  import Boot from "./lib/views/Boot.svelte";
  import Day from "./lib/views/Day.svelte";
  import CountriesDetail from "./lib/views/detail/CountriesDetail.svelte";
  import DaysDetail from "./lib/views/detail/DaysDetail.svelte";
  import EventsDetail from "./lib/views/detail/EventsDetail.svelte";
  import FollowersDetail from "./lib/views/detail/FollowersDetail.svelte";
  import LoadersDetail from "./lib/views/detail/LoadersDetail.svelte";
  import PlatformsDetail from "./lib/views/detail/PlatformsDetail.svelte";
  import ProjectsDetail from "./lib/views/detail/ProjectsDetail.svelte";
  import TimelineDetail from "./lib/views/detail/TimelineDetail.svelte";
  import Login from "./lib/views/Login.svelte";
  import ProjectDetail from "./lib/views/ProjectDetail.svelte";
  import Publish from "./lib/views/Publish.svelte";
  import Revenue from "./lib/views/Revenue.svelte";
  import Settings from "./lib/views/Settings.svelte";
  import Vision from "./lib/views/Vision.svelte";

  let view = $state<"vision" | "day" | "revenue" | "publish" | "settings">("vision");
  let ready = $state(false);

  $effect(() => {
    if (ready) return;
    ready = true;
    dashboard.start();
  });

  const overview = $derived(dashboard.overview);

  /** Durée des entrées et sorties, annulée si le système demande le calme. */
  const motion = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : 200;

  /**
   * Repli latéral, en ligne droite : le bloc rend sa largeur et s'efface, sans
   * jamais quitter son axe. Sans cette largeur animée, le reste de la barre
   * sauterait d'un coup à la fin de la disparition.
   */
  function retract(node: Element, { duration = motion }: { duration?: number } = {}) {
    const width = node.getBoundingClientRect().width;
    return {
      duration,
      easing: cubicOut,
      css: (t: number) =>
        `opacity:${t}; width:${t * width}px; overflow:hidden; white-space:nowrap;`,
    };
  }
</script>

<!--
  L'écran d'ouverture couvre la page, et s'efface une fois le relevé du jour
  rentré. Ce qui suit se rend dessous pendant qu'il disparaît : l'application
  ne passe donc jamais par une page vide.
-->
<Boot />

<!--
  L'application ne s'ouvre qu'avec ses deux comptes. Chartographer met les deux
  plateformes côte à côte : entrer avec une seule montrerait des totaux amputés
  sans qu'aucun chiffre ne signale ce qui manque.
-->
{#if !dashboard.auth}
  <!-- Les comptes n'ont pas encore répondu : l'écran d'ouverture tient la page. -->
{:else if !dashboard.auth.connected || dashboard.curseforgeSession !== true}
  <Login />
{:else if boot.done}
  <nav>
    <strong>Chartographer</strong>
    <button
      class:active={view === "vision"}
      onclick={() => {
        view = "vision";
        dashboard.closeDetail();
      }}
    >
      Vision
    </button>
    <button
      class:active={view === "day"}
      onclick={() => {
        view = "day";
        dashboard.closeDetail();
      }}
    >
      Journée
    </button>
    <button
      class:active={view === "revenue"}
      onclick={() => {
        view = "revenue";
        dashboard.closeDetail();
      }}
    >
      Revenus
    </button>
    <button
      class:active={view === "publish"}
      onclick={() => {
        view = "publish";
        dashboard.closeDetail();
      }}
    >
      Publication
    </button>
    <button class:active={view === "settings"} onclick={() => (view = "settings")}>Réglages</button>
    <span class="user">{dashboard.auth.username}</span>
    <!--
      Les deux filtres de plateforme n'agissent que sur les chiffres affichés :
      ils n'ont rien à faire sur la page des réglages, qui n'en montre aucun.
      Ils s'effacent donc, en glissant plutôt qu'en disparaissant d'un coup.
    -->
    {#if view !== "settings"}
      <span class="badges" transition:retract>
        <PlatformBadge
          platform="modrinth"
          account={dashboard.auth.username}
          count={dashboard.auth.modrinth_projects}
          active={dashboard.platforms.modrinth}
          ontoggle={() => dashboard.togglePlatform("modrinth")}
        />
        <PlatformBadge
          platform="curseforge"
          account={dashboard.auth.curseforge_username}
          count={dashboard.auth.curseforge_projects}
          active={dashboard.platforms.curseforge}
          ontoggle={() => dashboard.togglePlatform("curseforge")}
        />
      </span>
    {/if}
    <ThemeToggle />
  </nav>

  <main class:fixed={view === "settings"}>
    {#if dashboard.error}<p class="error">{dashboard.error}</p>{/if}
    {#if view === "settings"}
      <Settings />
    {:else if view === "day"}
      <Day />
    {:else if view === "revenue"}
      <Revenue />
    {:else if view === "publish"}
      <Publish />
    {:else}
      <Vision />
    {/if}
  </main>

  <!-- Vues plein écran, empilées par-dessus la page de vision. -->
  {#if dashboard.selectedProject}
    <ProjectDetail />
  {:else if dashboard.detail === "followers"}
    <!-- Les abonnés ne viennent pas de l'aperçu : cette vue les relève elle-même. -->
    <FollowersDetail />
  {:else if dashboard.detail === "days"}
    <!-- Le classement ne vient pas de l'aperçu non plus : il se relève lui-même. -->
    <DaysDetail />
  {:else if overview}
    {#if dashboard.detail === "timeline"}
      <TimelineDetail {overview} />
    {:else if dashboard.detail === "countries"}
      <CountriesDetail countries={overview.countries} />
    {:else if dashboard.detail === "platforms"}
      <PlatformsDetail projects={overview.per_project} />
    {:else if dashboard.detail === "loaders"}
      <LoadersDetail cells={overview.loaders} />
    {:else if dashboard.detail === "events"}
      <EventsDetail events={overview.events} />
    {:else if dashboard.detail === "projects"}
      <ProjectsDetail projects={overview.per_project} />
    {/if}
  {/if}
{/if}

<style>
  nav {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex-shrink: 0;
  }
  nav strong {
    margin-right: 14px;
    font-family: var(--font-display);
    font-size: 1.02rem;
    letter-spacing: 0.02em;
  }
  nav button {
    background: none;
    border: 0;
    color: var(--text-dim);
    font: inherit;
    font-size: 0.84rem;
    cursor: pointer;
    padding: 4px 9px;
    border-radius: var(--radius-sm);
  }
  nav button.active,
  nav button:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  /* Les deux pastilles glissent ensemble : elles forment un seul bloc, sinon
   * la barre se réorganiserait en deux temps. */
  .badges {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .user {
    margin-left: auto;
    color: var(--text-dim);
    font-size: 0.8rem;
    margin-right: 2px;
  }
  /*
   * La place de la barre de défilement est réservée en permanence. Sans cela,
   * la barre flottante de Windows apparaît au survol et disparaît ensuite, et
   * toute la colonne se décale de sa largeur à chaque passage de la souris.
   */
  main {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-gutter: stable;
    overscroll-behavior: contain;
    padding: 16px 18px 36px;
  }
  /* La vue des réglages gère son propre défilement, colonne par colonne. */
  main.fixed {
    overflow: hidden;
    padding-bottom: 16px;
  }
  .error {
    background: var(--surface-2);
    border-left: 2px solid var(--error);
    color: var(--error);
    border-radius: 0 var(--radius) var(--radius) 0;
    padding: 10px 14px;
    font-size: 0.84rem;
  }
</style>
