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

<div class="rounded-xl border border-border-subtle bg-surface p-4 group hover:border-border-default transition-colors">
  <div class="flex items-start justify-between">
    <p class="text-xs text-text-muted font-medium">{label}</p>
    {#if icon}
      <div class="w-8 h-8 rounded-lg bg-accent/8 flex items-center justify-center">
        <svg class="w-3.5 h-3.5 text-accent/70" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d={icon} />
        </svg>
      </div>
    {/if}
  </div>
  <p class="text-2xl font-bold text-text-primary tabular-nums tracking-tight mt-2">{value}</p>
  {#if subtitle}
    <p class="text-[11px] mt-1 {trendColors[trend]}">{subtitle}</p>
  {/if}
</div>
