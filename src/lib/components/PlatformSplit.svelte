<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { splitOption } from "../charts/split";
  import { palette } from "../charts/theme";
  import { theme } from "../theme.svelte";
  import type { ProjectSummary } from "../types";

  let { projects }: { projects: ProjectSummary[] } = $props();
  const option = $derived(splitOption(projects, palette(theme.dark)));

  /** Rien à comparer : aucun mod, ou aucun téléchargement sur aucun des deux. */
  const empty = $derived(
    projects.every((p) => p.modrinth_downloads + p.curseforge_downloads === 0),
  );
</script>

<!-- Les autres cartes de la page disent ce qui manque quand elles sont vides ;
     celle-ci laissait une légende et un axe suspendus au-dessus du vide. -->
{#if empty}
  <p class="empty">Aucun mod à comparer pour l'instant.</p>
{:else}
  <Chart {option} height="fill" />
{/if}

<style>
  .empty {
    color: var(--text-dim);
    font-size: 0.86rem;
    margin: 0;
    padding: 8px 0;
  }
</style>
