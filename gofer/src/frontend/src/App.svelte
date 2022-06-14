<script>
  import { onMount } from "svelte";
  import { globalHistory } from "svelte-routing/src/history";
  import { client } from "./client.js";
  import { AppStore } from "./store.js";

  let pathname = window.location.pathname;

  globalHistory.listen(({ location, _ }) => {
    pathname = location.pathname;
  });

  onMount(async () => {
    client.get_system_info().then((info) => {
      AppStore.backend_info.set(info);
    });
  });
</script>

<main class="mx-auto w-3/4 flex flex-col min-h-screen">
  <div class="flex-1">
    <h1 class="text-8xl">Gofer</h1>
    <p>Frontend not written yet</p>
  </div>
</main>

<style>
  :global(html) {
    overflow-y: scroll;
  }
</style>
