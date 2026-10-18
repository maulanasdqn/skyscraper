<script lang="ts">
  let {
    color = 'gray',
    text = '',
    dot = false,
  }: {
    color?: 'green' | 'yellow' | 'red' | 'blue' | 'gray' | 'purple' | 'orange'
    text: string
    dot?: boolean
  } = $props()

  const styles: Record<string, string> = {
    green: 'bg-success/10 text-success border-success/20',
    yellow: 'bg-warning/10 text-warning border-warning/20',
    red: 'bg-danger/10 text-danger border-danger/20',
    blue: 'bg-info/10 text-info border-info/20',
    gray: 'bg-surface-overlay text-text-muted border-border-subtle',
    purple: 'bg-accent/10 text-accent border-accent/20',
    orange: 'bg-orange-500/10 text-orange-400 border-orange-500/20',
  }

  const dots: Record<string, string> = {
    green: 'bg-success', yellow: 'bg-warning', red: 'bg-danger',
    blue: 'bg-info', gray: 'bg-text-muted', purple: 'bg-accent', orange: 'bg-orange-400',
  }
</script>

<span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-[11px] font-medium border {styles[color]}">
  {#if dot}<span class="w-1.5 h-1.5 rounded-full {dots[color]}"></span>{/if}
  {text}
</span>
