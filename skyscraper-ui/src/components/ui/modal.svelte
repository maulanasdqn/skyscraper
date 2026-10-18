<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    open = $bindable(false),
    title = '',
    children,
  }: {
    open?: boolean
    title?: string
    children?: Snippet
  } = $props()

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <div
      class="fixed inset-0 bg-black/70 backdrop-blur-sm animate-fade-in"
      role="button" tabindex="-1"
      onclick={() => (open = false)} onkeydown={onKeydown}
    ></div>

    <div class="relative z-10 w-full max-w-md mx-4 bg-surface border border-border-subtle rounded-2xl shadow-2xl shadow-black/40 animate-fade-up">
      <div class="flex items-center justify-between px-5 py-4 border-b border-border-subtle">
        <h3 class="text-sm font-semibold text-text-primary">{title}</h3>
        <button
          aria-label="Close"
          onclick={() => (open = false)}
          class="text-text-muted hover:text-text-primary transition-colors p-1.5 rounded-lg hover:bg-surface-overlay"
        >
          <svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor">
            <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
          </svg>
        </button>
      </div>
      <div class="p-5">
        {#if children}{@render children()}{/if}
      </div>
    </div>
  </div>
{/if}
