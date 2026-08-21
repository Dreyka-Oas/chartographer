<script lang="ts">
  /**
   * Une rangée de réglage : intitulé et explication à gauche, contrôle à
   * droite. C'est le seul endroit où la mise en page d'une rangée est décidée ;
   * les panneaux se contentent de fournir un contrôle.
   *
   * La grille et les hauteurs viennent des classes `stg-*` définies dans
   * `app.css` : la colonne de droite y a une largeur fixe, si bien qu'un
   * contrôle ne peut pas se déplacer parce que le texte a changé de longueur ou
   * qu'une barre de défilement est apparue.
   */
  import type { Snippet } from "svelte";

  let {
    name,
    desc = "",
    note = null,
    noteTone = "plain",
    control,
  }: {
    name: string;
    desc?: string;
    /**
     * Ligne d'appoint sous la description. `null` quand la rangée n'en a pas ;
     * une chaîne vide réserve la place d'une valeur encore en route, pour que
     * son arrivée ne pousse rien.
     */
    note?: string | null;
    /**
     * Un échec se voit. Écrite du même gris que l'explication au-dessus, une
     * signature refusée se lisait comme une note ordinaire, c'est pourtant le
     * seul message de ce panneau qui demande une réaction.
     */
    noteTone?: "plain" | "error";
    control?: Snippet;
  } = $props();
</script>

<div class="stg-row" class:wide={!control}>
  <div class="stg-text">
    <span class="stg-name">{name}</span>
    {#if desc}<span class="stg-desc">{desc}</span>{/if}
    {#if note !== null}<span class="stg-note" class:error={noteTone === "error"}>{note}</span>{/if}
  </div>
  {#if control}
    <div class="stg-control">{@render control()}</div>
  {/if}
</div>

<style>
  /* `.stg-note` vient de `app.css` ; seule la teinte change ici. */
  .stg-note.error {
    color: var(--error);
  }
</style>
