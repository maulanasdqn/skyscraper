<script lang="ts">
  let {
    label = '',
    value = '',
    subtitle = '',
    icon = '',
    trend = 'neutral',
  }: {
    label: string
    value: string | number
    subtitle?: string
    icon?: string
    trend?: 'up' | 'down' | 'neutral'
  } = $props()

  const trendColors: Record<string, string> = {
    up: 'text-success',
    down: 'text-danger',
    neutral: 'text-text-muted',
  }
</script>

<div class="rounded-xl border border-border-subtle bg-surface p-5">
  {#if icon}
    <div class="w-9 h-9 rounded-lg bg-accent-soft border border-border-subtle flex items-center justify-center mb-3">
      <svg class="w-4 h-4 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d={icon} />
      </svg>
    </div>
  {/if}
  <p class="text-xs text-text-muted mb-1.5">{label}</p>
  <p class="text-2xl font-bold text-text-primary tabular-nums tracking-tight">{value}</p>
  {#if subtitle}
    <p class="text-[11px] mt-1 {trendColors[trend]}">{subtitle}</p>
  {/if}
</div>
