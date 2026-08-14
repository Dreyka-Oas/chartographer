<script lang="ts">
  import * as echarts from "echarts";
  import { feature } from "topojson-client";
  import worldTopology from "world-atlas/countries-110m.json";
  import Chart from "../charts/Chart.svelte";
  import { palette } from "../charts/theme";
  import { worldMapOption } from "../charts/worldmap";
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

  const unknown = $derived(countries.find((c) => c.country === "??"));
  const option = $derived(worldMapOption(countries, palette(theme.dark)));
  const top = $derived(countries.filter((c) => c.country !== "??").slice(0, 6));
</script>

<Chart {option} height="fill" />

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
  .side {
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
