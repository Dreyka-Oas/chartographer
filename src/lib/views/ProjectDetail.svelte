<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { sparklineOption } from "../charts/sparkline";
  import { palette } from "../charts/theme";
  import Card from "../components/Card.svelte";
  import { compactNumber } from "../format";
  import { dashboard } from "../state.svelte";
  import { theme } from "../theme.svelte";

  const project = $derived(
    dashboard.overview?.per_project.find((p) => p.key === dashboard.selectedProject) ?? null,
  );
  const total = $derived(
    project ? project.modrinth_downloads + project.curseforge_downloads : 0,
  );
  const share = $derived(
    !project || total === 0 ? 0 : Math.round((project.modrinth_downloads / total) * 100),
  );
</script>

{#if project}
  <button class="back" onclick={() => (dashboard.selectedProject = null)}>← Retour</button>
  <h1>{project.title}</h1>

  <div class="grid">
    <Card title="Répartition par plateforme">
      <p class="big">{share} % Modrinth · {100 - share} % CurseForge</p>
      <p class="hint">
        {compactNumber(project.modrinth_downloads)} contre
        {compactNumber(project.curseforge_downloads)} téléchargements
      </p>
    </Card>

    <Card title="Tendance sur la période">
      {#if project.spark.length > 1}
        <Chart option={sparklineOption(project.spark, palette(theme.dark))} height={140} />
      {:else}
        <p class="hint">Pas encore assez de points.</p>
      {/if}
    </Card>

    <Card title="Appariement">
      {#if project.link_confidence === null}
        <p class="hint">
          Projet mono-plateforme. Apparie-le depuis les réglages si son jumeau existe.
        </p>
      {:else}
        <p class="big">{Math.round(project.link_confidence * 100)} %</p>
        <p class="hint">confiance de l'appariement automatique</p>
      {/if}
    </Card>
  </div>
{/if}

<style>
  .back {
    background: none;
    border: 0;
    color: var(--text-dim);
    font: inherit;
    cursor: pointer;
    padding: 0;
    margin-bottom: 8px;
  }
  h1 {
    font-size: 1.4rem;
    margin: 0 0 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 14px;
  }
  .big {
    font-size: 1.3rem;
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.82rem;
    margin: 6px 0 0;
  }
</style>
