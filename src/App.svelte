<script lang="ts">
  let view = $state<"vision" | "settings">("vision");
</script>

<main>
  <h1>Chartographer</h1>
  <p>Vue courante : {view}</p>
</main>
