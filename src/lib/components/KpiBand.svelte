<script lang="ts">
  import { compactNumber, deltaPercent, formatMoney } from "../format";
  import type { Kpis } from "../types";

  let { kpis }: { kpis: Kpis } = $props();

  const delta = $derived(deltaPercent(kpis.downloads_30d, kpis.downloads_prev_30d));
  const money = (raw: string) => Number.parseFloat(raw) || 0;

  /**
   * Chaque carte porte la même lecture : le total, puis d'où il vient. La barre
   * donne la proportion d'un coup d'œil, les deux mentions donnent le compte
   * exact. Une carte sans partage possible le dit plutôt que de laisser croire
   * à un total d'une seule source.
   */
  const tiles = $derived([
    {
      label: "Téléchargements",
      value: compactNumber(kpis.downloads_total),
      parts: {
        modrinth: kpis.downloads_modrinth,
        curseforge: kpis.downloads_curseforge,
        show: (v: number) => compactNumber(v),
      },
      hint: "",
    },
    {
      label: "30 derniers jours",
      value: compactNumber(kpis.downloads_30d),
      parts: {
        modrinth: kpis.downloads_30d_modrinth,
        curseforge: kpis.downloads_30d_curseforge,
        show: (v: number) => compactNumber(v),
      },
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
      parts: {
        modrinth: money(kpis.revenue_available_modrinth),
        curseforge: money(kpis.revenue_available_curseforge),
        show: (v: number) => formatMoney(String(v)),
      },
      hint: `${formatMoney(kpis.revenue_pending)} en maturation · onglet Revenus`,
    },
    {
      label: "Followers",
      value: compactNumber(kpis.followers),
      // CurseForge ne publie pas ses abonnés : la seule chose partageable ici
      // est le nombre de projets.
      parts: {
        modrinth: kpis.projects_modrinth,
        curseforge: kpis.projects_curseforge,
        show: (v: number) => `${v} projets`,
      },
      hint: "abonnés relevés sur Modrinth seul",
    },
  ]);

  function share(part: { modrinth: number; curseforge: number }) {
    const total = part.modrinth + part.curseforge;
    return total > 0 ? (part.modrinth / total) * 100 : 50;
  }
</script>

<div class="band">
  {#each tiles as tile (tile.label)}
    <article>
      <span class="label">{tile.label}</span>
      <strong>{tile.value}</strong>

      <div
        class="bar"
        title="{tile.parts.show(tile.parts.modrinth)} Modrinth · {tile.parts.show(
          tile.parts.curseforge,
        )} CurseForge"
      >
        <span class="modrinth" style="width:{share(tile.parts)}%"></span>
        <span class="curseforge"></span>
      </div>
      <div class="split">
        <span><i class="dot modrinth"></i>{tile.parts.show(tile.parts.modrinth)}</span>
        <span><i class="dot curseforge"></i>{tile.parts.show(tile.parts.curseforge)}</span>
      </div>

      {#if tile.hint}<span class="hint">{tile.hint}</span>{/if}
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
  /* Deux segments accolés : la part Modrinth donne la largeur, CurseForge
   * occupe ce qui reste. Aucun écart entre eux, c'est un seul tout partagé. */
  .bar {
    display: flex;
    height: 3px;
    margin: 4px 0 2px;
    border-radius: 999px;
    overflow: hidden;
    background: var(--surface-2);
  }
  .bar .modrinth {
    background: var(--modrinth);
    transition: width 260ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bar .curseforge {
    background: var(--curseforge);
    flex: 1;
  }
  .split {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    font-size: 0.76rem;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 2px;
    margin-right: 5px;
    vertical-align: middle;
  }
  .dot.modrinth {
    background: var(--modrinth);
  }
  .dot.curseforge {
    background: var(--curseforge);
  }
  .hint {
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  @media (prefers-reduced-motion: reduce) {
    .bar .modrinth {
      transition: none;
    }
  }
</style>
