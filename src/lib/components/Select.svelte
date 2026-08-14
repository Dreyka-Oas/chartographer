<script lang="ts">
  /**
   * Liste déroulante dessinée par l'application.
   *
   * Le `<select>` natif ne se laisse pas habiller : la liste qui s'ouvre est
   * rendue par Windows, hors de la page, et n'obéit ni aux couleurs du thème ni
   * à une hauteur maximale. On refait donc le déroulé : même comportement au
   * clavier, mais aux couleurs de la carte, et jamais plus de cinq entrées
   * visibles — au-delà, la liste défile.
   *
   * Le panneau est posé en `fixed` d'après la position du bouton : il échappe
   * ainsi aux colonnes qui défilent, qui l'auraient rogné.
   */
  export interface SelectOption {
    value: string | null;
    label: string;
  }

  let {
    value = $bindable<string | null>(null),
    options,
    disabled = false,
    label = "",
    align = "start",
    compact = false,
    onchange,
  }: {
    value: string | null;
    options: SelectOption[];
    disabled?: boolean;
    /** Nom accessible, quand aucun intitulé visible n'accompagne le champ. */
    label?: string;
    /** Bord sur lequel le panneau se cale, quand il est plus large que le bouton. */
    align?: "start" | "end";
    /** Format resserré, pour les barres d'outils. */
    compact?: boolean;
    /** Appelé après le choix, pour les champs pilotés depuis le parent. */
    onchange?: (value: string | null) => void;
  } = $props();

  /** Cinq entrées visibles, hauteur d'une entrée comprise. */
  const VISIBLE = 5;
  const ITEM = 30;

  let open = $state(false);
  let active = $state(0);
  let trigger = $state<HTMLButtonElement | null>(null);
  let panel = $state<HTMLDivElement | null>(null);
  let box = $state({ top: 0, left: 0, width: 0, up: false });

  const current = $derived(options.find((entry) => entry.value === value) ?? null);
  const shown = $derived(current?.label ?? options[0]?.label ?? "—");

  /** Place le panneau sous le bouton, ou au-dessus s'il n'y a pas la place. */
  function place() {
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const height = Math.min(options.length, VISIBLE) * ITEM + 10;
    const below = window.innerHeight - rect.bottom;
    const up = below < height + 12 && rect.top > below;
    box = {
      top: up ? rect.top - height - 6 : rect.bottom + 6,
      left: align === "end" ? rect.right : rect.left,
      width: rect.width,
      up,
    };
  }

  function toggle() {
    if (disabled) return;
    open = !open;
    if (open) {
      active = Math.max(
        0,
        options.findIndex((entry) => entry.value === value),
      );
      place();
      // Le panneau n'existe qu'une fois ouvert : on attend qu'il soit là pour
      // amener l'entrée courante sous les yeux.
      requestAnimationFrame(() => bring(active));
    }
  }

  function choose(entry: SelectOption) {
    value = entry.value;
    open = false;
    trigger?.focus();
    onchange?.(entry.value);
  }

  function onkeydown(event: KeyboardEvent) {
    if (disabled) return;
    if (!open && (event.key === "Enter" || event.key === " " || event.key === "ArrowDown")) {
      event.preventDefault();
      toggle();
      return;
    }
    if (!open) return;
    if (event.key === "Escape") {
      // Sans quoi l'évènement remonte jusqu'à la coque plein écran, qui se
      // ferme sur Échap sans regarder sa cible : fermer le menu fermerait la
      // page entière avec lui.
      event.preventDefault();
      event.stopPropagation();
      open = false;
      trigger?.focus();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      active = Math.min(options.length - 1, active + 1);
      reveal();
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      active = Math.max(0, active - 1);
      reveal();
    } else if (event.key === "Home") {
      event.preventDefault();
      active = 0;
      reveal();
    } else if (event.key === "End") {
      event.preventDefault();
      active = options.length - 1;
      reveal();
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      const entry = options[active];
      if (entry) choose(entry);
    }
  }

  /**
   * Amène une entrée dans la partie visible du panneau.
   *
   * `scrollIntoView` s'en chargerait, mais il remonte toute la chaîne des
   * ancêtres : quand l'entrée tenait déjà dans le panneau, il faisait tout de
   * même défiler la page derrière. Le bouton partait alors sous le panneau,
   * resté fixe. On ne touche donc qu'au défilement du panneau.
   */
  function bring(index: number) {
    const item = panel?.querySelectorAll<HTMLElement>("[role='option']")[index];
    if (!panel || !item) return;
    const top = item.offsetTop;
    const bottom = top + item.offsetHeight;
    if (top < panel.scrollTop) {
      panel.scrollTop = top;
    } else if (bottom > panel.scrollTop + panel.clientHeight) {
      panel.scrollTop = bottom - panel.clientHeight;
    }
  }

  function reveal() {
    requestAnimationFrame(() => bring(active));
  }

  /** Un clic ailleurs, un défilement de la page ou un redimensionnement referme.
   *
   * L'écoute du défilement est posée en capture pour attraper celui des
   * colonnes intérieures : il faut donc écarter explicitement le défilement de
   * la liste elle-même, sinon elle se referme dès qu'on la parcourt. */
  $effect(() => {
    if (!open) return;
    const inside = (event: Event) => {
      const target = event.target;
      if (!(target instanceof Node)) return false;
      return trigger?.contains(target) === true || panel?.contains(target) === true;
    };
    const away = (event: MouseEvent) => {
      if (inside(event)) return;
      open = false;
    };
    // Un défilement extérieur ne referme plus : le panneau suit le bouton. Il
    // ne se referme que si le bouton a quitté l'écran, faute de quoi la liste
    // flotterait toute seule.
    const rolled = (event: Event) => {
      if (inside(event)) return;
      const rect = trigger?.getBoundingClientRect();
      if (!rect || rect.bottom < 0 || rect.top > window.innerHeight) {
        open = false;
        return;
      }
      place();
    };
    const resized = () => (open = false);
    window.addEventListener("mousedown", away, true);
    window.addEventListener("scroll", rolled, true);
    window.addEventListener("resize", resized);
    return () => {
      window.removeEventListener("mousedown", away, true);
      window.removeEventListener("scroll", rolled, true);
      window.removeEventListener("resize", resized);
    };
  });
</script>

<button
  bind:this={trigger}
  type="button"
  class="trigger"
  class:open
  class:compact
  {disabled}
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-label={label || undefined}
  onclick={toggle}
  onkeydown={onkeydown}
>
  <span class="shown">{shown}</span>
  <span class="chevron" aria-hidden="true"></span>
</button>

{#if open}
  <div
    bind:this={panel}
    class="panel"
    class:up={box.up}
    role="listbox"
    tabindex="-1"
    aria-label={label || undefined}
    style="top:{box.top}px; {align === 'end'
      ? `left:auto; right:${window.innerWidth - box.left}px`
      : `left:${box.left}px`}; min-width:{box.width}px; max-height:{Math.min(
      options.length,
      VISIBLE,
    ) *
      ITEM +
      10}px"
  >
    {#each options as entry, index (entry.value ?? "__vide")}
      <div
        role="option"
        tabindex="-1"
        aria-selected={entry.value === value}
        data-active={index === active}
        class="option"
        class:selected={entry.value === value}
        class:active={index === active}
        onmouseenter={() => (active = index)}
        onclick={() => choose(entry)}
        onkeydown={onkeydown}
      >
        {entry.label}
      </div>
    {/each}
  </div>
{/if}

<style>
  .trigger {
    /* Peint après ses voisins : sans cela le trait du haut pouvait passer sous
     * la bordure de la ligne suivante, qui n'a pas de fond propre. */
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    /* Hauteur composée de nombres entiers : bordure, marges et hauteur de ligne
     * tombent ainsi sur des pixels pleins, et le trait du bas ne se perd plus
     * entre deux pixels physiques quand Windows applique une mise à l'échelle. */
    box-sizing: border-box;
    line-height: 18px;
    background-color: var(--surface-2);
    /* Le contour est une ombre intérieure, jamais une bordure : une bordure se
     * peint sur le bord de la boîte, qui peut tomber entre deux pixels
     * physiques une fois la mise à l'échelle de Windows appliquée — le trait du
     * bas disparaissait alors. L'ombre est peinte à l'intérieur, donc toujours
     * sur un pixel plein. */
    border: 0;
    box-shadow: inset 0 0 0 1px var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    font-size: 0.86rem;
    padding: 8px 11px;
    cursor: pointer;
    text-align: left;
    transition: box-shadow 120ms ease;
  }
  .trigger.compact {
    background-color: var(--surface);
    font-size: 0.8rem;
    line-height: 16px;
    padding: 6px 11px;
    gap: 8px;
  }
  .trigger:hover:not(:disabled),
  .trigger.open {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .trigger:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .shown {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Chevron dessiné en bordures : il suit la couleur du texte, sans image. */
  .chevron {
    width: 7px;
    height: 7px;
    border-right: 1.5px solid var(--text-dim);
    border-bottom: 1.5px solid var(--text-dim);
    /*
     * Le recentrage passe par le transform, jamais par une marge : une marge
     * qui change entre les deux états relance la mise en page du bouton, et le
     * libellé sautait d'un cran à l'ouverture.
     */
    transform: translateY(-2px) rotate(45deg);
    transition: transform 140ms ease;
    flex: none;
  }
  .trigger.open .chevron {
    transform: translateY(2px) rotate(-135deg);
  }
  .panel {
    position: fixed;
    z-index: 60;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 5px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: var(--lift);
    animation: drop 130ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .option {
    padding: 6px 9px;
    border-radius: 6px;
    font-size: 0.84rem;
    line-height: 1.2;
    color: var(--text);
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .option.active {
    background: var(--surface-2);
  }
  .option.selected {
    color: var(--accent);
    font-weight: 600;
  }
  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  .panel.up {
    animation-name: rise;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .panel {
      animation: none;
    }
    .chevron {
      transition: none;
    }
  }
</style>
