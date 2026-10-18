<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    title = '',
    children,
    noPadding = false,
  }: {
    title?: string
    children?: Snippet
    noPadding?: boolean
  } = $props()
</script>

<div class="rounded-xl border border-border-subtle bg-surface">
  {#if title}
    <div class="px-5 py-3.5 border-b border-border-subtle">
      <h3 class="text-[13px] font-semibold text-text-primary">{title}</h3>
    </div>
  {/if}
  <div class={noPadding ? '' : 'p-5'}>
    {#if children}{@render children()}{/if}
  </div>
</div>
