<script lang="ts">
  import * as echarts from "echarts";
  import { feature } from "topojson-client";
  import worldTopology from "world-atlas/countries-110m.json";
  import Chart from "../charts/Chart.svelte";
  import { palette } from "../charts/theme";
  import { fillZoom, worldMapOption } from "../charts/worldmap";
  import { compactNumber, countryLabel } from "../format";
  import { theme } from "../theme.svelte";
  import type { CountryTotal } from "../types";
  import { NUMERIC_TO_ALPHA2 } from "./iso";

  let { countries }: { countries: CountryTotal[] } = $props();

  // La geometrie de world-atlas porte des identifiants numeriques ISO-3166-1,
  // alors que Modrinth renvoie des codes alpha-2. On enregistre la carte une seule
  // fois en projetant chaque identifiant numerique vers son code alpha-2.
  let registered = false;

  /** Identifiant ISO-3166-1 numérique de l'Antarctique dans le fond de carte. */
  const ANTARCTICA = "010";

  function ensureMap() {
    if (registered) return;
    const topology = worldTopology as unknown as Parameters<typeof feature>[0] & {
      objects: { countries: Parameters<typeof feature>[1] };
    };
    const collection = feature(topology, topology.objects.countries) as unknown as {
      features: { id?: string | number; properties: Record<string, unknown> }[];
    };
    /*
     * L'Antarctique est retirée du fond de carte. Elle occupe toute la largeur
     * du bas — la projection l'étire démesurément — et ne porte jamais de
     * relevé : la garder tassait le monde habité dans la moitié haute du
     * panneau et offrait au survol une bulle « aucun téléchargement ».
     */
    collection.features = collection.features.filter(
      (item) => String(item.id ?? "").padStart(3, "0") !== ANTARCTICA,
    );
    for (const item of collection.features) {
      const numeric = String(item.id ?? "").padStart(3, "0");
      item.properties = { ...item.properties, iso_a2: NUMERIC_TO_ALPHA2[numeric] ?? numeric };
    }
    echarts.registerMap("world", collection as never);
    registered = true;
  }

  ensureMap();

  // Mesures du cadre : le grossissement de la carte s'y règle, pour qu'elle le
  // remplisse aussi bien dans la carte d'accueil que dans la vue dépliée.
  let width = $state(0);
  let height = $state(0);

  const unknown = $derived(countries.find((c) => c.country === "??"));
  const option = $derived(
    worldMapOption(countries, palette(theme.dark), fillZoom(width, height)),
  );
  const top = $derived(countries.filter((c) => c.country !== "??").slice(0, 6));
  /** Aucun pays situé : la carte n'aurait qu'une échelle graduée de zéro à un. */
  const empty = $derived(top.length === 0);
</script>

<!-- Une carte sans un seul pays coloré ne dit rien, et son échelle graduée
     « 0 — 1 » laisse croire à un relevé plutôt qu'à son absence. -->
{#if empty}
  <p class="empty">Aucune origine relevée. Modrinth les publie une fois les téléchargements comptés.</p>
{:else}
  <div class="canvas" bind:clientWidth={width} bind:clientHeight={height}>
    <Chart {option} height="fill" />
  </div>
{/if}

<div class="side">
  <ul>
    {#each top as row (row.country)}
      <li><span>{countryLabel(row.country)}</span><b>{compactNumber(row.downloads)}</b></li>
    {/each}
  </ul>
  {#if unknown}
    <p class="unknown">
      Origine inconnue : {compactNumber(unknown.downloads)} téléchargements, non représentés sur la
      carte.
    </p>
  {/if}
</div>

<style>
  .canvas {
    flex: 1;
    min-height: 220px;
    display: flex;
    flex-direction: column;
  }
  .empty {
    flex: 1;
    color: var(--text-dim);
    font-size: 0.86rem;
    margin: 0;
    padding: 8px 0;
  }
  .side {
    /* La liste garde sa taille : c'est la carte au-dessus qui absorbe le reste. */
    flex-shrink: 0;
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 4px 14px;
  }
  li {
    display: flex;
    justify-content: space-between;
    font-size: 0.82rem;
    color: var(--text-dim);
  }
  li b {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .unknown {
    margin: 0;
    font-size: 0.78rem;
    color: var(--warn);
  }
</style>
