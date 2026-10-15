<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    variant = 'primary',
    size = 'md',
    disabled = false,
    loading = false,
    onclick,
    children,
  }: {
    variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
    size?: 'sm' | 'md' | 'lg'
    disabled?: boolean
    loading?: boolean
    onclick?: (e: MouseEvent) => void
    children?: Snippet
  } = $props()

  const base = 'inline-flex items-center justify-center font-medium rounded-xl transition-all duration-150 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed'

  const variants: Record<string, string> = {
    primary: 'bg-accent text-white hover:bg-accent-hover active:scale-[0.98]',
    secondary: 'bg-surface-raised text-text-secondary hover:text-text-primary border border-border-subtle hover:border-border-default',
    danger: 'bg-danger/10 text-danger hover:bg-danger/15 border border-danger/15',
    ghost: 'text-text-muted hover:text-text-secondary hover:bg-surface-raised',
  }

  const sizes: Record<string, string> = {
    sm: 'px-2.5 py-1 text-xs gap-1.5',
    md: 'px-4 py-2 text-sm gap-2',
    lg: 'px-6 py-2.5 text-sm gap-2',
  }
</script>

<button
  class="{base} {variants[variant]} {sizes[size]}"
  disabled={disabled || loading}
  {onclick}
>
  {#if loading}
    <svg class="animate-spin h-3.5 w-3.5" viewBox="0 0 24 24" fill="none">
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
    </svg>
  {/if}
  {#if children}{@render children()}{/if}
</button>
