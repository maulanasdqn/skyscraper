<script lang="ts">
  let { title, subtitle }: { title: string; subtitle: string } = $props()
</script>

<div class="rounded-xl border border-border-subtle bg-surface p-6">
  <div class="flex items-center gap-4">
    <svg class="animate-spin h-4 w-4 text-accent shrink-0" viewBox="0 0 24 24" fill="none">
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
    </svg>
    <div>
      <p class="text-sm text-text-primary">{title}</p>
      <p class="text-xs text-text-muted mt-0.5">{subtitle}</p>
    </div>
  </div>
</div>
