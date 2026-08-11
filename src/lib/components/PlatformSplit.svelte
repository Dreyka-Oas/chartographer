<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { splitOption } from "../charts/split";
  import { palette } from "../charts/theme";
  import { theme } from "../theme.svelte";
  import type { ProjectSummary } from "../types";

  let { projects }: { projects: ProjectSummary[] } = $props();
  const option = $derived(splitOption(projects, palette(theme.dark)));
</script>

<Chart {option} height="fill" />
