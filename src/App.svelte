<script lang="ts">
  import { cubicOut } from "svelte/easing";
  import PlatformBadge from "./lib/components/PlatformBadge.svelte";
  import ThemeToggle from "./lib/components/ThemeToggle.svelte";
  import { dashboard } from "./lib/state.svelte";
  import CountriesDetail from "./lib/views/detail/CountriesDetail.svelte";
  import EventsDetail from "./lib/views/detail/EventsDetail.svelte";
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

  let view = $state<"vision" | "revenue" | "publish" | "settings">("vision");
  let ready = $state(false);

  $effect(() => {
    if (ready) return;
    ready = true;
    dashboard.boot();
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

{#if !dashboard.auth}
  <p class="boot">Démarrage…</p>
{:else if !dashboard.auth.connected}
  <Login />
{:else}
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
  main {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 16px 18px 36px;
  }
  /* La vue des réglages gère son propre défilement, colonne par colonne. */
  main.fixed {
    overflow: hidden;
    padding-bottom: 16px;
  }
  .boot {
    padding: 24px;
    color: var(--text-dim);
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
