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

  function onBackdropClick() {
    open = false
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <div
      class="fixed inset-0 bg-black/60 backdrop-blur-sm"
      role="button"
      tabindex="-1"
      onclick={onBackdropClick}
      onkeydown={onKeydown}
    ></div>

    <div class="relative z-10 w-full max-w-lg mx-4 bg-gray-900 border border-gray-700 rounded-xl shadow-2xl">
      <div class="flex items-center justify-between px-6 py-4 border-b border-gray-700">
        <h3 class="text-lg font-semibold text-gray-100">{title}</h3>
        <button
          aria-label="Close"
          onclick={() => (open = false)}
          class="text-gray-400 hover:text-gray-200 transition-colors"
        >
          <svg class="w-5 h-5" viewBox="0 0 20 20" fill="currentColor">
            <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
          </svg>
        </button>
      </div>

      <div class="px-6 py-4">
        {#if children}{@render children()}{/if}
      </div>
    </div>
  </div>
{/if}
