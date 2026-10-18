<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    icon = '',
    title = '',
    description = '',
    children,
  }: {
    icon?: string
    title: string
    description?: string
    children?: Snippet
  } = $props()
</script>

<div class="flex flex-col items-center justify-center py-16 px-4">
  {#if icon}
    <div class="w-12 h-12 rounded-2xl bg-surface-overlay border border-border-subtle flex items-center justify-center mb-4">
      <svg class="w-5 h-5 text-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d={icon} />
      </svg>
    </div>
  {/if}
  <p class="text-sm font-medium text-text-secondary">{title}</p>
  {#if description}
    <p class="text-xs text-text-muted mt-1 max-w-xs text-center">{description}</p>
  {/if}
  {#if children}
    <div class="mt-4">{@render children()}</div>
  {/if}
</div>
