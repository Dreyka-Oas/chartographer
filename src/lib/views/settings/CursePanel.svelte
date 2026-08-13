<script lang="ts">
  /** Section « CurseForge » : jeton d'envoi et points de collecte. */
  import { api } from "../../api";
  import CurseforgePoints from "../../components/CurseforgePoints.svelte";
  import { dashboard } from "../../state.svelte";
  import type { AppErrorPayload } from "../../types";
  import SettingRow from "./SettingRow.svelte";

  let { ready, onready }: { ready: boolean; onready: (value: boolean) => void } = $props();

  let capturing = $state(false);

  /**
   * Va chercher le jeton d'envoi sur le compte CurseForge. La fenêtre reste
   * cachée : elle ne s'ouvre que si la session a expiré.
   */
  async function capture() {
    capturing = true;
    try {
      onready(await api.captureCurseforgeToken());
    } catch (e) {
      dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
    } finally {
      capturing = false;
    }
  }
</script>

<section id="curseforge" class="stg-panel">
  <h2>CurseForge</h2>
  <SettingRow
    name="Jeton d'envoi"
    desc="Nécessaire pour publier un fichier. L'application en demande un à ton compte lors de sa première collecte, sous le nom « Chartographer », et ne l'affiche jamais. Tu peux le révoquer depuis CurseForge à tout moment."
  >
    {#snippet control()}
      <span class="stg-value">{ready ? "en place" : "absent"}</span>
      <button class="stg-btn" onclick={capture} disabled={capturing}>
        {capturing ? "Relevé en cours…" : "Relever"}
      </button>
    {/snippet}
  </SettingRow>
  <div class="stg-row wide">
    <CurseforgePoints />
  </div>
</section>
