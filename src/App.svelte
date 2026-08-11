<script lang="ts">
  import { dashboard } from "./lib/state.svelte";
  import Login from "./lib/views/Login.svelte";
  import ProjectDetail from "./lib/views/ProjectDetail.svelte";
  import Settings from "./lib/views/Settings.svelte";
  import Vision from "./lib/views/Vision.svelte";

  let view = $state<"vision" | "settings">("vision");
  let ready = $state(false);

  $effect(() => {
    if (ready) return;
    ready = true;
    dashboard.refreshAuth().then(() => {
      if (dashboard.auth?.connected) dashboard.load();
    });
  });
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
        dashboard.selectedProject = null;
      }}
    >
      Vision
    </button>
    <button class:active={view === "settings"} onclick={() => (view = "settings")}>Réglages</button>
    <span class="user">{dashboard.auth.username}</span>
  </nav>

  <main>
    {#if dashboard.error}<p class="error">{dashboard.error}</p>{/if}
    {#if view === "settings"}
      <Settings />
    {:else if dashboard.selectedProject}
      <ProjectDetail />
    {:else}
      <Vision />
    {/if}
  </main>
{/if}

<style>
  nav {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  nav strong {
    margin-right: 12px;
  }
  nav button {
    background: none;
    border: 0;
    color: var(--text-dim);
    font: inherit;
    font-size: 0.86rem;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
  }
  nav button.active,
  nav button:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  .user {
    margin-left: auto;
    color: var(--text-dim);
    font-size: 0.82rem;
  }
  main {
    padding: 18px 20px 40px;
  }
  .boot {
    padding: 24px;
    color: var(--text-dim);
  }
  .error {
    background: var(--surface-2);
    border: 1px solid var(--error);
    color: var(--error);
    border-radius: var(--radius);
    padding: 10px 14px;
    font-size: 0.84rem;
  }
</style>
