<script lang="ts">
  import { compactNumber, deltaPercent, formatMoney } from "../format";
  import type { Kpis } from "../types";

  let { kpis }: { kpis: Kpis } = $props();

  const delta = $derived(deltaPercent(kpis.downloads_30d, kpis.downloads_prev_30d));
  const tiles = $derived([
    {
      label: "Téléchargements",
      value: compactNumber(kpis.downloads_total),
      hint: `${compactNumber(kpis.downloads_modrinth)} Modrinth · ${compactNumber(kpis.downloads_curseforge)} CurseForge`,
    },
    {
      label: "30 derniers jours",
      value: compactNumber(kpis.downloads_30d),
      hint:
        delta === null
          ? "pas de période de référence"
          : `${delta > 0 ? "+" : ""}${delta} % vs 30 j précédents`,
    },
    {
      // Ce qui est déjà retiré appartient à l'onglet Revenus : en tête de page,
      // seul le solde encore disponible appelle une décision.
      label: "Revenus retirables",
      value: formatMoney(kpis.revenue_available),
      hint: `${formatMoney(kpis.revenue_pending)} en maturation · onglet Revenus`,
    },
    {
      label: "Followers",
      value: compactNumber(kpis.followers),
      hint: `${kpis.projects_active} projets actifs`,
    },
  ]);
</script>

<div class="band">
  {#each tiles as tile (tile.label)}
    <article>
      <span class="label">{tile.label}</span>
      <strong>{tile.value}</strong>
      <span class="hint">{tile.hint}</span>
    </article>
  {/each}
</div>

<style>
  .band {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: 12px;
  }
  article {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label {
    font-size: 0.75rem;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  strong {
    font-size: 1.7rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    font-size: 0.78rem;
    color: var(--text-dim);
  }
</style>
