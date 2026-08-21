<script lang="ts">
  import { compactNumber, deltaPercent, formatMoney } from "../format";
  import type { Kpis } from "../types";
  import Tooltip from "./Tooltip.svelte";

  let {
    kpis,
    ranged = $bindable<boolean[]>([]),
    days = 0,
    onfollowers,
  }: {
    kpis: Kpis;
    /** Lecture de chaque carte : vrai quand elle se rapporte à la période. */
    ranged?: boolean[];
    /** Nombre de jours de cette période, pour la moyenne quotidienne. */
    days?: number;
    /** Ouvre le détail nominatif des abonnés. */
    onfollowers?: () => void;
  } = $props();

  const delta = $derived(deltaPercent(kpis.downloads_30d, kpis.downloads_prev_30d));
  const rangeDelta = $derived(deltaPercent(kpis.range_downloads, kpis.range_downloads_prev));
  const money = (raw: string) => Number.parseFloat(raw) || 0;

  /** Part de la première plateforme, en pourcentage. Deux parts nulles se
   * partagent la barre en deux : mieux vaut une moitié franche qu'un trait
   * collé à un bord. */
  function shareOf(first: number, second: number) {
    const total = first + second;
    return total > 0 ? (first / total) * 100 : 50;
  }

  /**
   * La répartition dite en toutes lettres. La barre se partage en deux moitiés
   * quand rien n'a été relevé, mais l'écrire, "50 % Modrinth · 50 %
   * CurseForge" sous un total de zéro, affirmerait un partage qui n'existe
   * pas : sur une base neuve, la phrase annonçait une répartition inventée.
   */
  function splitHint(modrinth: number, curseforge: number) {
    if (modrinth + curseforge <= 0) return "aucun téléchargement relevé";
    const part = Math.round(shareOf(modrinth, curseforge));
    return `${part} % Modrinth · ${100 - part} % CurseForge`;
  }

  /**
   * Chaque carte porte la même lecture : le total, puis d'où il vient. La barre
   * donne la proportion d'un coup d'œil, les deux mentions donnent le compte
   * exact. Une carte sans partage possible le dit plutôt que de laisser croire
   * à un total d'une seule source.
   */
  /**
   * Les quatre mêmes cartes, rapportées à la période choisie.
   *
   * Deux d'entre elles n'ont pas d'équivalent honnête sur une fenêtre : un
   * solde retirable et un nombre d'abonnés sont des états de l'instant, dont
   * aucun historique quotidien n'est relevé. Plutôt que d'inventer une valeur,
   * elles changent de sujet, les revenus deviennent ceux gagnés sur la
   * période, les abonnés le disent et gardent leur compte du jour.
   */
  const perDay = (value: number) => (days > 0 ? Math.round(value / days) : 0);

  const rangeTiles = $derived([
    {
      label: "Téléchargements de la période",
      value: compactNumber(kpis.range_downloads),
      parts: {
        modrinth: kpis.range_downloads_modrinth,
        curseforge: kpis.range_downloads_curseforge,
        show: (v: number) => compactNumber(v),
      },
      hint:
        rangeDelta === null
          ? "pas de période de référence"
          : `${rangeDelta > 0 ? "+" : ""}${rangeDelta} % vs période précédente`,
    },
    {
      label: "Moyenne par jour",
      value: compactNumber(perDay(kpis.range_downloads)),
      parts: {
        modrinth: perDay(kpis.range_downloads_modrinth),
        curseforge: perDay(kpis.range_downloads_curseforge),
        show: (v: number) => compactNumber(v),
      },
      hint: `sur ${days} jour${days > 1 ? "s" : ""}`,
    },
    {
      label: "Revenus de la période",
      value: formatMoney(kpis.range_revenue),
      parts: {
        modrinth: money(kpis.range_revenue_modrinth),
        curseforge: money(kpis.range_revenue_curseforge),
        show: (v: number) => formatMoney(String(v)),
      },
      // CurseForge n'annonce qu'un solde de points : ce qu'il a rapporté sur la
      // période se retrouve par l'écart entre deux relevés, comme pour ses
      // téléchargements.
      hint: "Modrinth au jour le jour · CurseForge par écart de solde",
    },
    {
      label: "Followers",
      value: compactNumber(kpis.followers),
      parts: {
        modrinth: kpis.followers_modrinth,
        curseforge: kpis.followers_curseforge,
        show: (v: number) => compactNumber(v),
      },
      hint: "compte du jour : aucun historique d'abonnés n'est relevé",
    },
  ]);

  const stateTiles = $derived([
    {
      label: "Téléchargements",
      value: compactNumber(kpis.downloads_total),
      parts: {
        modrinth: kpis.downloads_modrinth,
        curseforge: kpis.downloads_curseforge,
        show: (v: number) => compactNumber(v),
      },
      // La carte n'a pas d'écart à commenter : elle dit la proportion, ce que
      // les deux montants seuls ne donnent pas d'un coup d'œil.
      hint: splitHint(kpis.downloads_modrinth, kpis.downloads_curseforge),
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
      parts: {
        modrinth: kpis.followers_modrinth,
        curseforge: kpis.followers_curseforge,
        show: (v: number) => compactNumber(v),
      },
      // CurseForge n'annonce qu'un total pour le compte : le dire évite de
      // chercher en vain le détail par mod.
      hint: `Modrinth par mod · CurseForge pour le compte entier`,
    },
  ]);

  /**
   * Chaque carte porte sa propre lecture : on veut souvent un cumul et une
   * période côte à côte. Les abonnés font exception, aucun historique n'en est
   * relevé, une version "période" serait le même chiffre sous un autre nom,
   * la carte n'a donc pas de bascule.
   */
  const tiles = $derived(
    stateTiles.map((tile, index) => ({
      ...(ranged[index] ? rangeTiles[index] : tile),
      switchable: index < 3,
      on: ranged[index] === true,
      // Les abonnés sont les seuls à avoir un détail nominatif : CurseForge les
      // nomme sur sa fiche publique.
      detail: index === 3,
    })),
  );

  function toggle(index: number) {
    ranged = ranged.map((value, i) => (i === index ? !value : value));
  }

  const scopeHint = (on: boolean) =>
    on
      ? "Cette carte suit les dates choisies. Cliquer pour revenir à l'état du compte."
      : "Cette carte montre l'état du compte. Cliquer pour la rapporter aux dates choisies.";

  const share = (part: { modrinth: number; curseforge: number }) =>
    shareOf(part.modrinth, part.curseforge);
</script>

<div class="band">
  {#each tiles as tile, index (index)}
    <article>
      <div class="top">
        <span class="label">{tile.label}</span>
        {#if tile.switchable}
          <!--
            La bascule ne porte pas de mot : la carte dit déjà ce qu'elle
            montre, et quatre libellés répétés encombreraient la bande. Le
            calendrier s'allume quand la carte suit les dates choisies, et la
            bulle dit le reste, sans attendre.

            Les abonnés n'en ont pas : ni Modrinth ni CurseForge ne disent qui
            suit un projet ni depuis quand, et aucun historique n'en est tenu
            ici. Une bascule qui ne change rien vaudrait moins que rien.
          -->
          <Tooltip text={scopeHint(tile.on)}>
            <button
              class="scope"
              class:on={tile.on}
              aria-pressed={tile.on}
              aria-label={scopeHint(tile.on)}
              onclick={() => toggle(index)}
            >
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <rect x="2.2" y="3.6" width="11.6" height="10.2" rx="2" />
                <path d="M2.2 6.8h11.6M5.6 2.2v2.8M10.4 2.2v2.8" />
              </svg>
            </button>
          </Tooltip>
        {:else if tile.detail}
          <Tooltip text="Voir qui te suit, nom par nom, avec la date d'arrivée constatée.">
            <button class="scope" aria-label="Voir le détail des abonnés" onclick={onfollowers}>
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <circle cx="6.2" cy="6" r="2.6" />
                <path d="M1.8 13.4c0-2.3 2-3.8 4.4-3.8s4.4 1.5 4.4 3.8M11 3.8a2.4 2.4 0 0 1 0 4.4M12.6 13.4c0-1.6-.7-2.7-1.8-3.3" />
              </svg>
            </button>
          </Tooltip>
        {/if}
      </div>
      <strong>{tile.value}</strong>

      <Tooltip
        block
        text="{tile.parts.show(tile.parts.modrinth)} Modrinth · {tile.parts.show(
          tile.parts.curseforge,
        )} CurseForge"
      >
        <!-- Rien de relevé : la barre s'éteint. Peinte à pleines couleurs sur
             deux moitiés, elle affirmait un partage que le libellé venait de
             démentir, et sur fond sombre, ces deux moitiés vives se lisaient
             de loin comme des chiffres. -->
        <div class="bar" class:blank={tile.parts.modrinth + tile.parts.curseforge <= 0}>
          <span class="modrinth" style="width:{share(tile.parts)}%"></span>
          <span class="curseforge"></span>
          <!-- Trait de partage : il dépasse en haut et en bas pour marquer la
               frontière sans dépendre du contraste entre les deux couleurs. -->
          <span class="cut" style="left:{share(tile.parts)}%"></span>
        </div>
      </Tooltip>
      <div class="split">
        <span><i class="dot modrinth"></i>{tile.parts.show(tile.parts.modrinth)}</span>
        <span><i class="dot curseforge"></i>{tile.parts.show(tile.parts.curseforge)}</span>
      </div>

      {#if tile.hint}<span class="hint">{tile.hint}</span>{/if}
    </article>
  {/each}
</div>

<style>
  /* Quatre tuiles : 1, 2 ou 4 colonnes, jamais 3, sinon la dernière rangée
   * n'en porte qu'une et laisse deux tiers de blanc. */
  .band {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
  }
  @media (min-width: 560px) {
    .band {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (min-width: 1100px) {
    .band {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
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
  /* L'intitulé et sa bascule partagent la première ligne : le bouton occupe la
   * place laissée à droite, sans jamais pousser le chiffre plus bas. */
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 20px;
  }
  /*
   * C'est la bulle qui enveloppe le bouton, donc elle qui est poussée à droite :
   * une marge posée sur le bouton lui-même ne ferait rien, il n'est pas l'enfant
   * de cette rangée. Les marges négatives la ramènent dans l'angle de la carte,
   * en reprenant le retrait que le padding lui donnait.
   */
  .top > :global(.anchor) {
    margin: -4px -6px -4px auto;
  }
  .label {
    font-size: 0.75rem;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    min-width: 0;
  }
  .scope {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: 6px;
    color: var(--text-dim);
    cursor: pointer;
    opacity: 0.55;
    transition:
      opacity 140ms ease,
      color 140ms ease,
      background-color 140ms ease;
  }
  .scope:hover {
    opacity: 1;
    background: var(--surface-2);
    color: var(--text);
  }
  .scope.on {
    opacity: 1;
    color: var(--accent);
  }
  .scope svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.4;
    stroke-linecap: round;
  }
  @media (prefers-reduced-motion: reduce) {
    .scope {
      transition: none;
    }
  }
  strong {
    font-size: 1.7rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  /* Deux segments accolés : la part Modrinth donne la largeur, CurseForge
   * occupe ce qui reste. Aucun écart entre eux, c'est un seul tout partagé. */
  .bar {
    position: relative;
    display: flex;
    height: 4px;
    /* Le trait dépasse de la barre : la marge du haut lui laisse la place. */
    margin: 8px 0 6px;
    border-radius: 999px;
    background: var(--surface-2);
  }
  .bar .modrinth {
    border-radius: 999px 0 0 999px;
  }
  .bar .curseforge {
    border-radius: 0 999px 999px 0;
  }
  .cut {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--text);
    transition: left 260ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bar .modrinth {
    background: var(--modrinth);
    transition: width 260ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bar .curseforge {
    background: var(--curseforge);
    flex: 1;
  }
  /* La piste reste, les couleurs et le trait de partage s'en vont : il n'y a
   * pas de partage à montrer. */
  .bar.blank .modrinth,
  .bar.blank .curseforge {
    background: transparent;
  }
  .bar.blank .cut {
    display: none;
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
    .bar .modrinth,
    .cut {
      transition: none;
    }
  }
</style>
