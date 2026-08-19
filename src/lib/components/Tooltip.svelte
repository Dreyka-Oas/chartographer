<script lang="ts">
  /**
   * Bulle d'aide, posée au survol de ce qu'elle explique.
   *
   * L'attribut `title` du navigateur ferait le même office, mais il se fait
   * attendre une bonne seconde, s'affiche aux couleurs de Windows et se place
   * sous le curseur plutôt que contre l'élément. Celle-ci paraît tout de suite,
   * suit le thème, et se cale au-dessus — ou en dessous s'il n'y a pas la
   * place.
   *
   * La bulle est posée en `fixed`, d'après la position de l'élément : elle
   * échappe ainsi aux cartes et aux colonnes qui défilent, qui l'auraient
   * rognée.
   *
   * Elle enveloppe ce qu'elle décrit :
   *
   * ```svelte
   * <Tooltip text="Ce que fait ce bouton">
   *   <button aria-label="Ce que fait ce bouton">…</button>
   * </Tooltip>
   * ```
   *
   * Le nom accessible reste porté par l'élément enveloppé : la bulle est un
   * confort de lecture, pas le seul moyen de savoir à quoi l'on touche.
   */
  import type { Snippet } from "svelte";
  import { anchorX, placeBubble } from "./tooltipPlacement";

  let {
    text,
    /** Bord privilégié. La bulle bascule d'elle-même si la place manque. */
    placement = "top",
    /**
     * L'enveloppe s'aligne par défaut comme un mot, ce qui convient aux boutons
     * et aux étiquettes. Une barre ou un bloc qui tenait toute la largeur la
     * perdrait en devenant du texte : `block` la lui rend.
     */
    block = false,
    children,
  }: {
    text: string;
    placement?: "top" | "bottom";
    block?: boolean;
    children: Snippet;
  } = $props();

  /**
   * Largeur supposée avant la première mesure. La bulle est en réalité plus
   * large que ce maximum — s'y ajoutent son cadre et ses marges intérieures —
   * et c'est la valeur mesurée qui compte ensuite.
   */
  const WIDTH = 282;

  let anchor = $state<HTMLSpanElement | null>(null);
  let bubble = $state<HTMLDivElement | null>(null);
  let open = $state(false);
  let box = $state({ top: 0, left: 0, below: false });

  /**
   * Place la bulle. Sans hauteur connue, elle se pose du côté demandé ; une
   * fois peinte, l'effet ci-dessous la mesure et la corrige si elle déborde.
   */
  function place(height: number, width: number) {
    if (!anchor) return;
    const rect = anchor.getBoundingClientRect();
    const spot = placeBubble(rect, height, window.innerHeight, placement);
    box = {
      top: spot.top,
      below: spot.below,
      left: anchorX(rect.left + rect.width / 2, width, window.innerWidth),
    };
  }

  /** Numéro d'ouverture, pour qu'une fermeture différée ne referme pas une
   * bulle rouverte entre-temps. */
  let opened = 0;

  function show() {
    if (!text) return;
    place(0, WIDTH);
    opened += 1;
    open = true;
  }

  /**
   * Corrige le placement une fois la bulle peinte.
   *
   * Ses dimensions dépendent du texte : les supposer rognait les explications
   * un peu longues contre le haut de la fenêtre, et débordait d'un cheveu sur
   * les bords, le cadre et les marges s'ajoutant à la largeur maximale. On les
   * mesure donc, et on rejoue la même règle avec les vraies valeurs.
   */
  $effect(() => {
    if (!open || !bubble) return;
    place(bubble.offsetHeight, bubble.offsetWidth);
  });

  /**
   * Un élément arraché sous la souris émet un dernier `mouseleave`, en pleine
   * destruction de son bloc — là où Svelte refuse toute écriture d'état
   * (`state_unsafe_mutation`). La fermeture attend donc la microtâche
   * suivante, hors de cette fenêtre ; à l'œil, rien ne change.
   */
  function hide() {
    if (!open) return;
    const generation = opened;
    queueMicrotask(() => {
      if (generation === opened) open = false;
    });
  }

  /**
   * Les écouteurs sont posés sur l'élément plutôt que déclarés en attributs.
   * Ce conteneur n'est pas une commande — il n'a ni rôle ni action propre, il
   * ne fait qu'observer le passage de la souris au-dessus de ce qu'il enveloppe.
   * L'écrire ainsi évite de lui prêter une interactivité qu'il n'a pas.
   */
  $effect(() => {
    const element = anchor;
    if (!element) return;
    element.addEventListener("mouseenter", show);
    element.addEventListener("mouseleave", hide);
    element.addEventListener("focusin", show);
    element.addEventListener("focusout", hide);
    return () => {
      element.removeEventListener("mouseenter", show);
      element.removeEventListener("mouseleave", hide);
      element.removeEventListener("focusin", show);
      element.removeEventListener("focusout", hide);
    };
  });

  /**
   * La bulle ne suit pas ce qui bouge : elle disparaît. Un défilement ou un
   * changement de taille la laisserait flotter loin de son élément, et la
   * repositionner à chaque image coûterait plus que de la rouvrir.
   */
  $effect(() => {
    if (!open) return;
    window.addEventListener("scroll", hide, true);
    window.addEventListener("resize", hide);
    return () => {
      window.removeEventListener("scroll", hide, true);
      window.removeEventListener("resize", hide);
    };
  });
</script>

<span bind:this={anchor} class="anchor" class:block>
  {@render children()}
</span>

{#if open && text}
  <div
    bind:this={bubble}
    class="bubble"
    class:below={box.below}
    role="tooltip"
    style="top:{box.top}px; left:{box.left}px"
  >
    {text}
  </div>
{/if}

<style>
  .anchor {
    display: inline-flex;
  }
  .anchor.block {
    display: block;
    width: 100%;
  }
  .bubble {
    position: fixed;
    z-index: 80;
    /* Le point d'ancrage est le milieu de l'élément : la bulle se centre
     * dessus, et se hisse au-dessus d'elle-même quand elle le surplombe. */
    transform: translate(-50%, -100%);
    /*
     * `max-content` avant tout : sans elle, la largeur d'une boîte posée en
     * `fixed` se limite à la place restant à droite de son `left`. Une bulle
     * ancrée près du bord droit se repliait alors en une colonne d'un mot par
     * ligne, le décalage de moitié n'intervenant qu'après la mise en page.
     */
    width: max-content;
    max-width: min(260px, calc(100vw - 20px));
    padding: 7px 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: var(--lift);
    color: var(--text);
    font-size: 0.78rem;
    line-height: 1.4;
    text-align: left;
    /* Les sauts de ligne du texte sont respectés, les longues lignes se
     * replient toujours : une explication en deux temps se sépare ainsi d'un
     * blanc, au lieu de couler en un seul pavé. */
    white-space: pre-line;
    /* Elle ne doit jamais intercepter la souris : passer dessus reviendrait à
     * quitter l'élément, et la bulle se refermerait aussitôt. */
    pointer-events: none;
    animation: appear 90ms ease-out;
  }
  .bubble.below {
    transform: translate(-50%, 0);
  }
  @keyframes appear {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bubble {
      animation: none;
    }
  }
</style>
