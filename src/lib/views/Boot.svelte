<script lang="ts">
  /**
   * Écran d'ouverture. Il couvre la page tant que le relevé du jour n'est pas
   * rentré, et rend compte de chaque étape plutôt que de faire tourner un
   * sablier : le cycle complet demande plusieurs dizaines de secondes, et une
   * attente muette de cette longueur ne se distingue pas d'un blocage.
   */
  import { cubicOut } from "svelte/easing";
  import { boot } from "../boot.svelte";
  import { dashboard } from "../state.svelte";

  const calm = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /** Les quatre courbes du relevé, de la plus large à la plus resserrée. */
  const CONTOURS = [
    "M100 20 C136 20 172 48 176 92 C180 136 148 176 104 179 C58 182 24 150 21 106 C18 60 56 20 100 20 Z",
    "M101 44 C131 42 156 68 157 100 C158 132 134 155 103 156 C70 157 45 132 44 101 C43 70 70 46 101 44 Z",
    "M100 68 C122 66 136 84 135 101 C134 120 121 133 101 133 C82 133 66 119 66 100 C66 82 80 69 100 68 Z",
    "M100 88 C110 87 114 94 113 101 C112 109 108 112 100 112 C92 112 87 107 87 100 C87 93 92 89 100 88 Z",
  ];

  /** Sortie : l'écran s'éloigne d'un souffle et découvre la page dessous. */
  function leave(_node: Element) {
    return {
      duration: calm ? 0 : 420,
      easing: cubicOut,
      css: (t: number, u: number) => `opacity: ${t}; transform: scale(${1 + u * 0.03});`,
    };
  }

  /** Le démarrage est en panne quand les comptes eux-mêmes n'ont pas répondu. */
  const stuck = $derived(boot.steps[0].state === "failed");

  /**
   * Passé ce délai, une porte de sortie s'ouvre. Le cycle complet demande une
   * demi-minute sur un compte fourni ; au-delà, c'est qu'une source traîne, et
   * mieux vaut la page d'hier tout de suite qu'une attente sans fin.
   */
  let waited = $state(false);
  $effect(() => {
    const timer = setTimeout(() => (waited = true), 45_000);
    return () => clearTimeout(timer);
  });
</script>

{#if !boot.done}
  <div class="opening" out:leave>
    <div class="stage">
      <svg class="rose" viewBox="0 0 200 200" aria-hidden="true">
        <defs>
          <linearGradient
            id="boot-sweep"
            x1="100"
            y1="100"
            x2="100"
            y2="16"
            gradientUnits="userSpaceOnUse"
          >
            <stop class="from" offset="0" />
            <stop class="to" offset="1" />
          </linearGradient>
        </defs>
        <!--
          Le relevé reste dessiné en entier ; ce sont des arcs qui courent
          dessus, chacun à sa vitesse. Un tracé qui s'effacerait à chaque tour
          laisserait le dessin vide une fraction de seconde sur deux.
        -->
        {#each CONTOURS as contour, i (contour)}
          <path class="contour" d={contour} pathLength="1" />
          <path
            class="arc"
            d={contour}
            pathLength="1"
            style="animation-duration: {2600 + i * 700}ms; animation-delay: -{i * 400}ms"
          />
        {/each}
        <line class="needle" x1="100" y1="100" x2="100" y2="16" stroke="url(#boot-sweep)" />
        <circle class="pin modrinth" cx="157" cy="100" r="2.6" />
        <circle class="pin curseforge" cx="44" cy="101" r="2.6" style="animation-delay: 900ms" />
        <circle class="center" cx="100" cy="100" r="2.4" />
      </svg>

      <h1>Chartographer</h1>
      <p class="doing" class:halted={stuck}>
        {stuck ? "Démarrage interrompu" : boot.label}
      </p>

      <div class="track">
        <div class="fill" style="transform: scaleX({boot.progress})"></div>
      </div>

      <ol class="steps">
        {#each boot.steps as step, i (step.key)}
          <li class={step.state} style="--i: {i}">
            <span class="mark"></span>
            <span class="label">{step.label}</span>
            <span class="note">{step.note}</span>
          </li>
        {/each}
      </ol>

      {#if dashboard.error}
        <p class="error">{dashboard.error}</p>
      {/if}
      {#if stuck}
        <button onclick={() => dashboard.start()}>Reprendre le démarrage</button>
      {:else if waited}
        <button class="link" onclick={() => boot.release()}>
          Entrer sans attendre la fin du relevé
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  /*
   * L'écran se pose par-dessus tout le reste : la page se rend dessous pendant
   * qu'il s'efface, ce qui enchaîne les deux sans passage par le vide.
   */
  .opening {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-content: center;
    justify-items: center;
    background-color: var(--bg);
    background-image: radial-gradient(120% 70% at 50% -10%, var(--halo), transparent 70%);
  }
  .stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: min(520px, 90vw);
  }

  .rose {
    width: 168px;
    height: 168px;
    overflow: visible;
  }
  .contour {
    fill: none;
    stroke: var(--rule);
    stroke-width: 1.2;
    opacity: 0.5;
  }
  .arc {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.6;
    stroke-linecap: round;
    /* Un tiers de la courbe, qui en fait le tour sans fin. */
    stroke-dasharray: 0.3 0.7;
    animation: turn linear infinite;
  }
  /* La couleur du balayage est posée ici plutôt qu'en attribut : elle suit le
   * thème, et un attribut de présentation ne lit pas les variables du même
   * œil selon le moteur. */
  .from,
  .to {
    stop-color: var(--accent);
  }
  .from {
    stop-opacity: 0;
  }
  .to {
    stop-opacity: 0.55;
  }
  .needle {
    stroke-width: 1.4;
    transform-origin: 100px 100px;
    animation: sweep 6s linear infinite;
  }
  .center {
    fill: var(--accent);
  }
  /* Deux repères posés sur le relevé, aux couleurs des deux plateformes : ce
   * sont elles que l'écran est en train d'interroger. */
  .pin {
    animation: blip 2.4s ease-in-out infinite;
  }
  .pin.modrinth {
    fill: var(--modrinth);
  }
  .pin.curseforge {
    fill: var(--curseforge);
  }

  h1 {
    margin: 10px 0 0;
    font-family: var(--font-display);
    font-size: 1.6rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    animation: rise 520ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  /* La ligne garde sa hauteur d'un intitulé à l'autre : le bloc entier
   * sauterait à chaque changement d'étape. */
  .doing {
    margin: 3px 0 16px;
    min-height: 1.3em;
    font-size: 0.82rem;
    color: var(--text-dim);
  }
  .doing.halted {
    color: var(--error);
  }

  .track {
    width: 100%;
    height: 3px;
    border-radius: 3px;
    background: var(--border);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transform-origin: left;
    transform: scaleX(0);
    transition: transform 640ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .steps {
    list-style: none;
    margin: 18px 0 0;
    padding: 0;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  /* L'intitulé garde sa largeur, le compte rendu occupe ce qui reste et se
   * cale à droite : les notes forment ainsi une colonne, quelle que soit la
   * longueur des libellés. */
  li {
    display: grid;
    grid-template-columns: 14px auto minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    font-size: 0.84rem;
    color: var(--text-dim);
    animation: rise 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--i) * 45ms + 140ms);
  }
  .mark {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--border);
    display: grid;
    place-items: center;
    transition:
      background-color 220ms ease,
      box-shadow 220ms ease;
  }
  /* Étape en cours : un arc tourne dans l'anneau. Le masque évide le disque,
   * il ne reste donc que le filet. */
  li.running .mark {
    box-shadow: none;
    background: conic-gradient(from 0deg, transparent 15%, var(--accent));
    -webkit-mask: radial-gradient(farthest-side, transparent calc(100% - 2px), #000 0);
    mask: radial-gradient(farthest-side, transparent calc(100% - 2px), #000 0);
    animation: spin 900ms linear infinite;
  }
  li.done .mark {
    background: var(--accent);
    box-shadow: none;
  }
  li.done .mark::after {
    content: "";
    width: 3.5px;
    height: 7px;
    margin-top: -2px;
    border: solid var(--on-accent);
    border-width: 0 1.6px 1.6px 0;
    transform: rotate(45deg);
    animation: pop 260ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  li.failed .mark {
    box-shadow: inset 0 0 0 1.5px var(--error);
  }
  li.failed .mark::after {
    content: "×";
    color: var(--error);
    font-size: 0.78rem;
    line-height: 1;
  }
  li.running .label,
  li.done .label {
    color: var(--text);
  }
  li.failed .label {
    color: var(--error);
  }
  .note {
    justify-self: end;
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-dim);
    opacity: 0.75;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error {
    margin: 18px 0 0;
    max-width: 52ch;
    text-align: center;
    font-size: 0.82rem;
    color: var(--error);
  }
  button {
    margin-top: 12px;
    background: var(--accent);
    color: var(--on-accent);
    border: 0;
    border-radius: var(--radius-sm);
    padding: 8px 18px;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 600;
    cursor: pointer;
  }
  /* La porte de sortie ne se dispute pas la vedette avec le relevé en cours. */
  button.link {
    margin-top: 22px;
    background: none;
    color: var(--text-dim);
    font-weight: 400;
    font-size: 0.78rem;
    padding: 4px 8px;
    text-decoration: underline;
  }
  button.link:hover {
    color: var(--accent);
  }

  @keyframes turn {
    from {
      stroke-dashoffset: 1;
    }
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes sweep {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes blip {
    0%,
    100% {
      opacity: 0.25;
    }
    50% {
      opacity: 1;
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: rotate(45deg) scale(0.4);
    }
    to {
      opacity: 1;
      transform: rotate(45deg) scale(1);
    }
  }

  /*
   * Calme demandé : plus rien ne tourne ni ne se trace. Le relevé reste
   * dessiné, la barre saute d'une étape à l'autre, et l'écran dit toujours
   * où il en est.
   */
  @media (prefers-reduced-motion: reduce) {
    .needle,
    .pin,
    li,
    h1,
    li.running .mark,
    li.done .mark::after {
      animation: none;
    }
    /* Le relevé reste, les arcs qui le parcourent s'en vont. */
    .arc {
      display: none;
    }
    .contour {
      opacity: 1;
    }
    .fill {
      transition: none;
    }
    li.running .mark {
      background: var(--surface-2);
      box-shadow: inset 0 0 0 1.5px var(--accent);
    }
  }
</style>
