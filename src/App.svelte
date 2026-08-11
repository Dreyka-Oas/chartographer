<script lang="ts">
  import ThemeToggle from "./lib/components/ThemeToggle.svelte";
  import { dashboard } from "./lib/state.svelte";
  import CountriesDetail from "./lib/views/detail/CountriesDetail.svelte";
  import EventsDetail from "./lib/views/detail/EventsDetail.svelte";
  import LoadersDetail from "./lib/views/detail/LoadersDetail.svelte";
  import PlatformsDetail from "./lib/views/detail/PlatformsDetail.svelte";
  import ProjectsDetail from "./lib/views/detail/ProjectsDetail.svelte";
  import RevenueDetail from "./lib/views/detail/RevenueDetail.svelte";
  import TimelineDetail from "./lib/views/detail/TimelineDetail.svelte";
  import Login from "./lib/views/Login.svelte";
  import ProjectDetail from "./lib/views/ProjectDetail.svelte";
  import Settings from "./lib/views/Settings.svelte";
  import Vision from "./lib/views/Vision.svelte";

  let view = $state<"vision" | "settings">("vision");
  let ready = $state(false);

  $effect(() => {
    if (ready) return;
    ready = true;
    dashboard.boot();
  });

  const overview = $derived(dashboard.overview);
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
    <button class:active={view === "settings"} onclick={() => (view = "settings")}>Réglages</button>
    <span class="user">{dashboard.auth.username}</span>
    <ThemeToggle />
  </nav>

  <main>
    {#if dashboard.error}<p class="error">{dashboard.error}</p>{/if}
    {#if view === "settings"}
      <Settings />
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
    {:else if dashboard.detail === "revenue"}
      <RevenueDetail {overview} />
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
    padding: 11px 22px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
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
  .user {
    margin-left: auto;
    color: var(--text-dim);
    font-size: 0.8rem;
  }
  main {
    /* Au-delà, les cartes s'étirent en bandes trop larges pour être lisibles. */
    max-width: 1760px;
    margin: 0 auto;
    padding: 18px 22px 42px;
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
