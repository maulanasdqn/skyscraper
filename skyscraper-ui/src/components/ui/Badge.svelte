<script lang="ts">
  let {
    color = 'gray',
    text = '',
  }: {
    color?: 'green' | 'yellow' | 'red' | 'blue' | 'gray' | 'purple'
    text: string
  } = $props()

  const colors: Record<string, string> = {
    green: 'bg-green-900/50 text-green-400 ring-green-500/30',
    yellow: 'bg-yellow-900/50 text-yellow-400 ring-yellow-500/30',
    red: 'bg-red-900/50 text-red-400 ring-red-500/30',
    blue: 'bg-blue-900/50 text-blue-400 ring-blue-500/30',
    gray: 'bg-gray-800 text-gray-400 ring-gray-600/30',
    purple: 'bg-purple-900/50 text-purple-400 ring-purple-500/30',
  }
</script>

<span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ring-1 ring-inset {colors[color]}">
  {text}
</span>
