<script lang="ts">
  /** Section "Compte Modrinth" : état du token et sortie de session. */
  import { api } from "../../api";
  import Confirm from "../../components/Confirm.svelte";
  import { formatDayLong } from "../../format";
  import { dashboard } from "../../state.svelte";
  import SettingRow from "./SettingRow.svelte";

  const auth = $derived(dashboard.auth);
  const since = $derived(
    auth?.connected_since ? formatDayLong(auth.connected_since.slice(0, 10)) : null,
  );

  /** La déconnexion efface le token et vide les relevés : elle se confirme. */
  let leaving = $state(false);
</script>

<section id="compte" class="stg-panel">
  <h2>Compte Modrinth</h2>
  {#if auth?.connected}
    <SettingRow
      name="Session"
      desc="Le token ne quitte jamais cette machine : aucune requête ne part depuis la fenêtre."
      note={since ? `Token enregistré le ${since}.` : ""}
    >
      {#snippet control()}
        <button class="stg-btn" onclick={() => api.openTokenPage()}>Gérer mes tokens</button>
        <button class="stg-btn danger" onclick={() => (leaving = true)}>Se déconnecter</button>
      {/snippet}
    </SettingRow>
  {:else}
    <SettingRow name="Aucun token" desc="Reconnecte-toi pour relancer les relevés." />
  {/if}
</section>

<Confirm
  bind:open={leaving}
  title="Se déconnecter de Modrinth ?"
  body="Le token enregistré sur cette machine est effacé et les relevés affichés disparaissent. L'historique déjà collecté, lui, reste en place : il revient à la reconnexion."
  confirmLabel="Se déconnecter"
  danger
  onconfirm={() => dashboard.logout()}
/>
