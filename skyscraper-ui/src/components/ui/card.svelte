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

<div class="rounded-xl border border-border-subtle bg-surface">
  {#if title}
    <div class="px-5 py-3.5 border-b border-border-subtle">
      <h3 class="text-sm font-semibold text-text-primary">{title}</h3>
    </div>
  {/if}
  <div class="p-5">
    {#if children}{@render children()}{/if}
  </div>
</div>
