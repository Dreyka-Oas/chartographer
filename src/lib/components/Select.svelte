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
      requestAnimationFrame(() => {
        panel?.querySelector<HTMLElement>("[data-active='true']")?.scrollIntoView({
          block: "nearest",
        });
      });
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
      event.preventDefault();
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

  function reveal() {
    requestAnimationFrame(() => {
      panel
        ?.querySelectorAll<HTMLElement>("[role='option']")
        [active]?.scrollIntoView({ block: "nearest" });
    });
  }

  /** Un clic ailleurs, un défilement ou un redimensionnement referme. */
  $effect(() => {
    if (!open) return;
    const away = (event: MouseEvent) => {
      const target = event.target as Node;
      if (trigger?.contains(target) || panel?.contains(target)) return;
      open = false;
    };
    const follow = () => (open = false);
    window.addEventListener("mousedown", away, true);
    window.addEventListener("scroll", follow, true);
    window.addEventListener("resize", follow);
    return () => {
      window.removeEventListener("mousedown", away, true);
      window.removeEventListener("scroll", follow, true);
      window.removeEventListener("resize", follow);
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
    display: inline-flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    background-color: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    font-size: 0.86rem;
    padding: 7px 10px;
    cursor: pointer;
    text-align: left;
    transition: border-color 120ms ease;
  }
  .trigger.compact {
    background-color: var(--surface);
    font-size: 0.8rem;
    padding: 5px 10px;
    gap: 8px;
  }
  .trigger:hover:not(:disabled),
  .trigger.open {
    border-color: var(--accent);
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
    margin-top: -3px;
    border-right: 1.5px solid var(--text-dim);
    border-bottom: 1.5px solid var(--text-dim);
    transform: rotate(45deg);
    transition: transform 140ms ease;
    flex: none;
  }
  .trigger.open .chevron {
    transform: rotate(-135deg);
    margin-top: 2px;
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
