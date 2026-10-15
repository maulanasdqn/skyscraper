<script lang="ts">
  let {
    color = 'gray',
    text = '',
  }: {
    color?: 'green' | 'yellow' | 'red' | 'blue' | 'gray' | 'purple' | 'orange'
    text: string
  } = $props()

  const colors: Record<string, string> = {
    green: 'bg-success/10 text-success',
    yellow: 'bg-warning/10 text-warning',
    red: 'bg-danger/10 text-danger',
    blue: 'bg-info/10 text-info',
    gray: 'bg-surface-overlay text-text-muted',
    purple: 'bg-accent/10 text-accent',
    orange: 'bg-orange-500/10 text-orange-400',
  }
</script>

<span class="inline-flex items-center px-2 py-0.5 rounded-md text-[11px] font-medium {colors[color]}">
  {text}
</span>
