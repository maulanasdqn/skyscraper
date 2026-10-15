<script lang="ts">
  import type { Snippet } from 'svelte'

  type Column = {
    key: string
    label: string
    render?: Snippet<[unknown, Record<string, unknown>]>
  }

  let {
    columns = [],
    rows = [],
    loading = false,
  }: {
    columns: Column[]
    rows: Record<string, unknown>[]
    loading?: boolean
  } = $props()
</script>

<div class="overflow-x-auto rounded-lg border border-gray-800">
  <table class="min-w-full divide-y divide-gray-800">
    <thead class="bg-gray-900/80">
      <tr>
        {#each columns as col}
          <th class="px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider">
            {col.label}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody class="divide-y divide-gray-800/50 bg-gray-900/30">
      {#if loading}
        <tr>
          <td colspan={columns.length} class="px-4 py-8 text-center text-sm text-gray-500">
            <svg class="animate-spin h-5 w-5 mx-auto text-indigo-500" viewBox="0 0 24 24" fill="none">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
          </td>
        </tr>
      {:else if rows.length === 0}
        <tr>
          <td colspan={columns.length} class="px-4 py-8 text-center text-sm text-gray-500">
            No data
          </td>
        </tr>
      {:else}
        {#each rows as row, i}
          <tr class="{i % 2 === 0 ? 'bg-gray-900/20' : ''} hover:bg-gray-800/50 transition-colors">
            {#each columns as col}
              <td class="px-4 py-3 text-sm text-gray-300 whitespace-nowrap">
                {#if col.render}
                  {@render col.render(row[col.key], row)}
                {:else}
                  {row[col.key] ?? '—'}
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
</div>
