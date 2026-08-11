<script lang="ts">
  import StatRow from "../../components/StatRow.svelte";
  import ProjectsTable from "../../components/ProjectsTable.svelte";
  import { compactNumber } from "../../format";
  import { dashboard } from "../../state.svelte";
  import type { ProjectSummary } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let { projects }: { projects: ProjectSummary[] } = $props();

  let search = $state("");

  const rows = $derived(
    search.trim()
      ? projects.filter((p) => p.title.toLowerCase().includes(search.trim().toLowerCase()))
      : projects,
  );

  const total = $derived(
    projects.reduce((s, p) => s + p.modrinth_downloads + p.curseforge_downloads, 0),
  );
  const followers = $derived(projects.reduce((s, p) => s + p.followers, 0));
  const weakLinks = $derived(
    projects.filter((p) => p.link_confidence !== null && p.link_confidence < 1).length,
  );
  const solo = $derived(
    projects.filter((p) => p.modrinth_id === null || p.curseforge_id === null).length,
  );
</script>

<DetailShell title="Tous les projets" subtitle="{projects.length} entrées · clique une ligne pour le détail">
  {#snippet actions()}
    <input bind:value={search} placeholder="Filtrer par nom…" />
  {/snippet}

  <StatRow
    stats={[
      { label: "Téléchargements cumulés", value: compactNumber(total) },
      { label: "Followers", value: compactNumber(followers) },
      { label: "Mono-plateforme", value: String(solo), hint: "sans jumeau apparié" },
      { label: "Appariements fragiles", value: String(weakLinks), hint: "confiance sous 100 %" },
    ]}
  />

  <div class="panel">
    <ProjectsTable projects={rows} onselect={(key) => {
      const found = projects.find((p) => p.key === key);
      if (found) dashboard.openProject(found);
    }} />
  </div>
</DetailShell>

<style>
  input {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    padding: 5px 9px;
    font: inherit;
    font-size: 0.8rem;
    width: 200px;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
</style>
