<script lang="ts">
  import Chart from "../../charts/Chart.svelte";
  import { rankingOption } from "../../charts/multiseries";
  import { palette } from "../../charts/theme";
  import Hint from "../../components/Hint.svelte";
  import RankedTable from "../../components/RankedTable.svelte";
  import StatRow from "../../components/StatRow.svelte";
  import WorldMap from "../../components/WorldMap.svelte";
  import { compactNumber, countryLabel, formatPercent } from "../../format";
  import { theme } from "../../theme.svelte";
  import type { CountryTotal } from "../../types";

  import DetailShell from "./DetailShell.svelte";

  let { countries }: { countries: CountryTotal[] } = $props();

  const known = $derived(countries.filter((c) => c.country !== "??"));
  const unknown = $derived(countries.find((c) => c.country === "??"));
  const totalKnown = $derived(known.reduce((s, c) => s + c.downloads, 0));
  const top = $derived([...known].slice(0, 15).reverse());

  const option = $derived(
    rankingOption(
      top.map((c) => countryLabel(c.country)),
      top.map((c) => c.downloads),
      palette(theme.dark),
    ),
  );

  const leader = $derived(known[0]);

  /**
   * Le titre compte ce que le graphique montre vraiment. "Top 15" écrit
   * au-dessus de quatre barres promet onze lignes qui ne viendront pas, et
   * laisse croire à un chargement inabouti.
   */
  const topTitle = $derived(
    top.length >= 15 ? "Les quinze premiers" : "Du plus gros au plus petit",
  );
</script>

<DetailShell
  title="Origine des téléchargements"
  subtitle="{known.length} pays identifiés · données Modrinth uniquement"
>
  <StatRow
    stats={[
      { label: "Pays identifiés", value: String(known.length) },
      {
        label: "Téléchargements localisés",
        value: compactNumber(totalKnown),
      },
      {
        label: "Premier pays",
        value: leader ? countryLabel(leader.country) : "—",
        hint: leader
          ? `${compactNumber(leader.downloads)} · ${Math.round((leader.downloads / totalKnown) * 100)} % du localisé`
          : undefined,
      },
      {
        label: "Origine inconnue",
        value: unknown ? compactNumber(unknown.downloads) : "0",
        hint: "non représentés sur la carte",
      },
    ]}
  />

  <div class="grid">
    <div class="panel map">
      <WorldMap {countries} />
    </div>

    <div class="panel">
      <h2>{topTitle}</h2>
      <!-- Sans pays, le graphique ne montrait qu'un axe vertical suspendu. -->
      {#if top.length === 0}
        <p class="empty">Aucune origine relevée pour l'instant.</p>
      {:else}
        <Chart {option} height={430} />
      {/if}
    </div>
  </div>

  <div class="panel wide">
    <h2>
      Tous les pays
      <Hint
        text="Tous les pays relevés, du plus gros au plus petit, les trois premiers marqués d'un rang coloré. La part se lit sur le total localisé, hors téléchargements d'origine inconnue. Ces origines viennent de Modrinth seul : CurseForge n'en publie aucune."
      />
    </h2>
    <RankedTable
      columns={[
        { label: "Pays", align: "left" },
        { label: "Code" },
        { label: "Téléchargements" },
        { label: "Part" },
      ]}
      rows={known}
      empty="Aucune origine relevée. Modrinth les publie une fois les téléchargements comptés."
      key={(row) => row.country}
      maxHeight={420}
    >
      {#snippet cells(row)}
        <td class="left">{countryLabel(row.country)}</td>
        <td class="dim">{row.country}</td>
        <td>{compactNumber(row.downloads)}</td>
        <td class="share">
          <div class="bar">
            <span style="width: {totalKnown ? (row.downloads / totalKnown) * 100 : 0}%"></span>
          </div>
          {formatPercent(row.downloads, totalKnown)} %
        </td>
      {/snippet}
    </RankedTable>
    {#if unknown}
      <p class="unknown">
        Origine inconnue : {compactNumber(unknown.downloads)} téléchargements, hors carte et hors
        classement.
      </p>
    {/if}
  </div>
</DetailShell>

<style>
  .grid {
    display: grid;
    /* La carte n'est pas beaucoup plus large que haute : lui donner la moitié
     * du rang plutôt que les trois cinquièmes lui évite deux marges vides, et
     * le classement à côté n'en est que plus lisible. */
    grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr);
    gap: 14px;
  }
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  /*
   * Le panneau de la carte est étiré à la hauteur du Top 15 par la grille, mais
   * la carte ne s'y étendait pas : `Chart height="fill"` se règle en `flex: 1`,
   * qui ne veut rien dire hors d'un conteneur flex. Le panneau en devient un, et
   * la carte prend tout ce que la liste des pays lui laisse.
   */
  .map {
    display: flex;
    flex-direction: column;
  }
  .wide {
    margin-top: 14px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 10px;
    font-size: 0.9rem;
    font-weight: 600;
  }
  /* Filets, alignements, rangs et survol viennent de `RankedTable` : ne reste
   * ici que la barre de part, propre à ce tableau. */
  .share {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .bar {
    width: 110px;
    height: 6px;
    border-radius: 3px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .unknown {
    margin: 10px 0 0;
    font-size: 0.8rem;
    color: var(--warn);
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.86rem;
    margin: 0;
    padding: 8px 0;
  }
  @media (max-width: 1100px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
