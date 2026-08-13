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

  let {
    text,
    /** Bord privilégié. La bulle bascule d'elle-même si la place manque. */
    placement = "top",
    children,
  }: {
    text: string;
    placement?: "top" | "bottom";
    children: Snippet;
  } = $props();

  /** Marge entre l'élément et la bulle, et garde contre les bords de la vue. */
  const GAP = 8;
  const EDGE = 10;
  /** Hauteur supposée pour décider du côté, sans avoir à mesurer la bulle. */
  const ROOM = 44;
  /**
   * Largeur maximale de la bulle, reprise telle quelle dans la feuille de
   * style. Sa moitié sert à écarter le point d'ancrage des bords : la bulle est
   * centrée dessus, elle déborde donc de part et d'autre.
   */
  const WIDTH = 260;

  let anchor = $state<HTMLSpanElement | null>(null);
  let open = $state(false);
  let box = $state({ top: 0, left: 0, below: false });

  function place() {
    if (!anchor) return;
    const rect = anchor.getBoundingClientRect();
    const below = placement === "bottom" ? rect.bottom + ROOM < window.innerHeight : rect.top < ROOM;
    // Le point d'ancrage est le milieu de l'élément, écarté des bords de la
    // moitié de la bulle : sans cette garde, une bulle collée au bord droit se
    // voyait rognée.
    const half = Math.min(WIDTH, window.innerWidth - 2 * EDGE) / 2;
    const middle = rect.left + rect.width / 2;
    const room = Math.max(window.innerWidth - EDGE - half, EDGE + half);
    box = {
      top: below ? rect.bottom + GAP : rect.top - GAP,
      left: Math.min(Math.max(middle, EDGE + half), room),
      below,
    };
  }

  function show() {
    if (!text) return;
    place();
    open = true;
  }

  const hide = () => (open = false);

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

<span bind:this={anchor} class="anchor">
  {@render children()}
</span>

{#if open && text}
  <div
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
