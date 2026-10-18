<script lang="ts">
  import { getToasts, dismissToast } from '@lib/stores/toast.svelte'

  const iconPaths: Record<string, string> = {
    success: 'M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z',
    error: 'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z',
    info: 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z',
  }

  const colors: Record<string, string> = {
    success: 'text-success',
    error: 'text-danger',
    info: 'text-info',
  }
</script>

<div class="fixed top-4 right-4 z-[100] flex flex-col gap-2 pointer-events-none">
  {#each getToasts() as toast (toast.id)}
    <div
      class="pointer-events-auto flex items-start gap-3 px-4 py-3 rounded-xl
        bg-surface-overlay/95 backdrop-blur-lg border border-border-subtle
        shadow-2xl shadow-black/30 max-w-sm animate-toast-in"
      role="alert"
    >
      <svg class="w-4 h-4 mt-0.5 shrink-0 {colors[toast.type]}" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d={iconPaths[toast.type]} />
      </svg>
      <p class="text-sm text-text-primary flex-1">{toast.message}</p>
      <button onclick={() => dismissToast(toast.id)}
        class="text-text-muted hover:text-text-secondary transition-colors shrink-0 mt-0.5"
      >
        <svg class="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor">
          <path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" />
        </svg>
      </button>
    </div>
  {/each}
</div>
