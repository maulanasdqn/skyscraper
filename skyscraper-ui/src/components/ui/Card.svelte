<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    title = '',
    children,
  }: {
    title?: string
    children?: Snippet
  } = $props()
</script>

<div class="rounded-xl border border-gray-800 bg-gray-900/50 shadow-sm">
  {#if title}
    <div class="px-5 py-3 border-b border-gray-800">
      <h3 class="text-sm font-semibold text-gray-200">{title}</h3>
    </div>
  {/if}
  <div class="p-5">
    {#if children}{@render children()}{/if}
  </div>
</div>
