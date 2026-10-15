<script lang="ts">
  import type { Snippet } from 'svelte'
  import Sidebar from './sidebar.svelte'

  let {
    currentRoute = $bindable('playground'),
    children,
  }: {
    currentRoute?: string
    children?: Snippet
  } = $props()
</script>

<div class="flex h-screen bg-[#0c0c14] text-text-primary">
  <Sidebar bind:currentRoute />
  <main class="flex-1 overflow-y-auto">
    <div class="max-w-6xl mx-auto px-8 py-8">
      {#if children}{@render children()}{/if}
    </div>
  </main>
</div>
