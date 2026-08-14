<script lang="ts">
  /**
   * Champ de date dessiné par l'application.
   *
   * Le panneau qu'ouvre un `<input type="date">` est rendu par Windows, hors
   * de la page : fond blanc, coins carrés, aucune prise sur ses couleurs. On
   * refait donc le calendrier, sur le même parti pris que `Select.svelte` — le
   * champ le plus proche dans ce dépôt à avoir affronté le même problème —
   * pour que les trois champs de dates de l'application partagent un seul
   * calendrier, aux couleurs de la carte plutôt qu'à celles du système.
   *
   * Le panneau est posé en `fixed` d'après la position du bouton : il échappe
   * ainsi aux colonnes qui défilent, qui l'auraient rogné.
   */
  import { formatDayLong, formatMonth, lastDayOfMonth } from "../format";
  import { addDays, monthGrid, shiftMonth } from "./calendar";

  let {
    value = $bindable<string>(""),
    min = "",
    max = "",
    label = "",
    onchange,
  }: {
    value: string;
    /** Première date sélectionnable, incluse ; "" pour aucune borne. */
    min?: string;
    /** Dernière date sélectionnable, incluse ; "" pour aucune borne. */
    max?: string;
    /** Nom accessible, quand aucun intitulé visible n'accompagne le champ. */
    label?: string;
    /** Appelé après le choix, pour les champs pilotés depuis le parent. */
    onchange?: (value: string) => void;
  } = $props();

  const WEEKDAYS = ["lu", "ma", "me", "je", "ve", "sa", "di"];

  /** Dimensions du panneau, dessin exact de son style : elles servent au
   * placement, calculé avant que le panneau n'existe dans le DOM. */
  const PANEL_WIDTH = 260;
  const PANEL_HEIGHT = 262;

  const todayIso = new Date().toISOString().slice(0, 10);

  let open = $state(false);
  let month = $state(todayIso.slice(0, 7));
  let activeDay = $state(todayIso);
  let trigger = $state<HTMLButtonElement | null>(null);
  let panel = $state<HTMLDivElement | null>(null);
  let box = $state({ top: 0, left: 0, up: false });

  const grid = $derived(monthGrid(month));
  const shown = $derived(value ? formatDayLong(value) : "—");

  function isDisabled(day: string): boolean {
    return (min !== "" && day < min) || (max !== "" && day > max);
  }

  /** Place le panneau sous le bouton, ou au-dessus s'il n'y a pas la place. */
  function place() {
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom;
    const up = below < PANEL_HEIGHT + 12 && rect.top > below;
    box = {
      top: up ? rect.top - PANEL_HEIGHT - 6 : rect.bottom + 6,
      // Calé à gauche du bouton, mais ramené dans la fenêtre s'il en dépasserait.
      left: Math.min(rect.left, window.innerWidth - PANEL_WIDTH - 8),
      up,
    };
  }

  function toggle() {
    open = !open;
    if (open) {
      month = (value || todayIso).slice(0, 7);
      activeDay = value || todayIso;
      place();
    }
  }

  function choose(day: string) {
    if (isDisabled(day)) return;
    value = day;
    open = false;
    trigger?.focus();
    onchange?.(day);
  }

  /** Change de mois en gardant le même quantième, ramené au dernier jour du
   * mois visé s'il n'existe pas (31 janvier -> 28 ou 29 février). */
  function changeMonth(by: number) {
    const next = shiftMonth(month, by);
    const lastDay = Number(lastDayOfMonth(next).slice(8));
    const day = Math.min(Number(activeDay.slice(8)), lastDay);
    month = next;
    activeDay = `${next}-${String(day).padStart(2, "0")}`;
  }

  /** Déplace le jour actif ; ne change de mois que si le déplacement sort de
   * la grille affichée, qui montre déjà les débords des mois voisins. */
  function moveActive(by: number) {
    const next = addDays(activeDay, by);
    activeDay = next;
    if (!grid.some((week) => week.includes(next))) {
      month = next.slice(0, 7);
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (!open && (event.key === "Enter" || event.key === " " || event.key === "ArrowDown")) {
      event.preventDefault();
      toggle();
      return;
    }
    if (!open) return;
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        open = false;
        trigger?.focus();
        break;
      case "ArrowLeft":
        event.preventDefault();
        moveActive(-1);
        break;
      case "ArrowRight":
        event.preventDefault();
        moveActive(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        moveActive(-7);
        break;
      case "ArrowDown":
        event.preventDefault();
        moveActive(7);
        break;
      case "PageUp":
        event.preventDefault();
        changeMonth(-1);
        break;
      case "PageDown":
        event.preventDefault();
        changeMonth(1);
        break;
      case "Enter":
        event.preventDefault();
        choose(activeDay);
        break;
      default:
        break;
    }
  }

  /** Un clic ailleurs, un défilement de la page ou un redimensionnement
   * referme — voir Select.svelte, qui pose la même écoute pour la même
   * raison : le panneau suit le bouton tant qu'il reste à l'écran. */
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
  aria-label={label || undefined}
  aria-haspopup="dialog"
  aria-expanded={open}
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
    style="top:{box.top}px; left:{box.left}px; width:{PANEL_WIDTH}px"
  >
    <div class="head">
      <button type="button" class="nav" aria-label="Mois précédent" onclick={() => changeMonth(-1)}>
        ‹
      </button>
      <span class="title">{formatMonth(month)}</span>
      <button type="button" class="nav" aria-label="Mois suivant" onclick={() => changeMonth(1)}>
        ›
      </button>
    </div>
    <div class="weekdays" aria-hidden="true">
      {#each WEEKDAYS as initial (initial)}
        <span>{initial}</span>
      {/each}
    </div>
    <div class="grid" role="grid" aria-label={label || "Calendrier"}>
      {#each grid as week, weekIndex (weekIndex)}
        <div class="week" role="row">
          {#each week as day (day)}
            <div
              role="gridcell"
              tabindex="-1"
              aria-selected={day === value}
              aria-disabled={isDisabled(day) || undefined}
              class="day"
              class:muted={!day.startsWith(month)}
              class:selected={day === value}
              class:today={day === todayIso}
              class:active={day === activeDay}
              class:disabled={isDisabled(day)}
              onclick={() => choose(day)}
              onmouseenter={() => (activeDay = day)}
              onkeydown={onkeydown}
            >
              {Number(day.slice(8))}
            </div>
          {/each}
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .trigger {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    box-sizing: border-box;
    line-height: 18px;
    background-color: var(--surface);
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
    font-size: 0.8rem;
    padding: 5px 10px;
    cursor: pointer;
    text-align: left;
    font-variant-numeric: tabular-nums;
    transition: box-shadow 120ms ease;
  }
  .trigger:hover,
  .trigger.open {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .shown {
    white-space: nowrap;
  }
  /* Chevron dessiné en bordures : il suit la couleur du texte, sans image. */
  .chevron {
    width: 7px;
    height: 7px;
    border-right: 1.5px solid var(--text-dim);
    border-bottom: 1.5px solid var(--text-dim);
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
    box-sizing: border-box;
    padding: 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: var(--lift);
    animation: drop 130ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .nav {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: 0;
    border-radius: 6px;
    color: var(--text-dim);
    font-size: 1rem;
    cursor: pointer;
  }
  .nav:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .title {
    font-size: 0.84rem;
    font-weight: 600;
    color: var(--text);
    text-transform: capitalize;
  }
  .weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    margin-bottom: 4px;
  }
  .weekdays span {
    text-align: center;
    font-size: 0.66rem;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .grid {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .week {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
  }
  .day {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 30px;
    border-radius: 6px;
    font-size: 0.82rem;
    color: var(--text);
    cursor: pointer;
    font-variant-numeric: tabular-nums;
  }
  .day:hover:not(.disabled) {
    background: var(--surface-2);
  }
  .day.active:not(.disabled) {
    background: var(--surface-2);
  }
  .day.muted {
    color: var(--text-dim);
  }
  .day.selected {
    color: var(--accent);
    font-weight: 600;
  }
  /* Liseré du jour courant : une ombre intérieure, pour la même raison que le
   * contour du bouton — toujours peinte sur un pixel plein. */
  .day.today {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .day.disabled {
    color: var(--text-dim);
    opacity: 0.4;
    cursor: default;
    pointer-events: none;
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
