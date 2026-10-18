<script lang="ts">
  let { title = '', subtitle = '' }: { title?: string; subtitle?: string } = $props()
</script>

<div class="rounded-xl border border-border-subtle bg-surface p-6 animate-fade-in">
  <div class="flex items-center gap-4">
    <div class="relative w-5 h-5 shrink-0">
      <svg class="animate-spin h-5 w-5 text-accent" viewBox="0 0 24 24" fill="none">
        <circle class="opacity-20" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2.5" />
        <path class="opacity-80" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
    </div>
    <div class="min-w-0">
      {#if title}
        <p class="text-sm font-medium text-text-primary">{title}</p>
      {/if}
      {#if subtitle}
        <p class="text-xs text-text-muted mt-0.5">{subtitle}</p>
      {/if}
    </div>
  </div>
</div>
