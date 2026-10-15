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

<div class="overflow-x-auto">
  <table class="w-full">
    <thead>
      <tr class="border-b border-border-subtle">
        {#each columns as col}
          <th class="px-3 py-2.5 text-left text-[11px] font-medium text-text-muted uppercase tracking-wider">
            {col.label}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr>
          <td colspan={columns.length} class="px-3 py-8 text-center">
            <svg class="animate-spin h-4 w-4 mx-auto text-accent" viewBox="0 0 24 24" fill="none">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
          </td>
        </tr>
      {:else if rows.length === 0}
        <tr>
          <td colspan={columns.length} class="px-3 py-8 text-center text-sm text-text-muted">
            No data
          </td>
        </tr>
      {:else}
        {#each rows as row}
          <tr class="border-b border-border-subtle/50 hover:bg-surface-raised/50 transition-colors">
            {#each columns as col}
              <td class="px-3 py-2.5 text-sm text-text-secondary">
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
