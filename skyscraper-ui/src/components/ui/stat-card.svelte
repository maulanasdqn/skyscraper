<script lang="ts">
  let {
    label = '',
    value = '',
    subtitle = '',
    trend = 'neutral',
  }: {
    label: string
    value: string | number
    subtitle?: string
    trend?: 'up' | 'down' | 'neutral'
  } = $props()

  const trendColors: Record<string, string> = {
    up: 'text-green-400',
    down: 'text-red-400',
    neutral: 'text-gray-500',
  }

  const trendIcons: Record<string, string> = {
    up: '↑',
    down: '↓',
    neutral: '—',
  }
</script>

<div class="rounded-xl border border-gray-800 bg-gray-900/50 p-5">
  <p class="text-xs font-medium text-gray-500 uppercase tracking-wider">{label}</p>
  <div class="mt-2 flex items-baseline gap-2">
    <p class="text-2xl font-bold text-gray-100">{value}</p>
    {#if subtitle}
      <span class="text-xs {trendColors[trend]}">
        {trendIcons[trend]} {subtitle}
      </span>
    {/if}
  </div>
</div>
