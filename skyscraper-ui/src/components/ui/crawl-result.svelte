<script lang="ts">
  import { formatMs } from '@lib/utils/format'
  import type { CrawlResponse } from '@lib/types'

  let {
    crawl,
    wallTimeMs = 0,
  }: {
    crawl: CrawlResponse
    wallTimeMs?: number
  } = $props()
</script>

<div class="rounded-xl border border-border-subtle bg-surface overflow-hidden">
  <div class="flex items-center justify-between px-4 py-2.5 border-b border-border-subtle">
    <div class="flex items-center gap-3 text-sm">
      <span class="w-1.5 h-1.5 rounded-full bg-success"></span>
      <span class="text-text-primary font-semibold">{crawl.pages_discovered} pages discovered</span>
      {#if wallTimeMs > 0}
        <span class="text-text-muted text-xs">{formatMs(wallTimeMs)}</span>
      {/if}
    </div>
    <a href="#jobs" class="text-[11px] text-accent hover:text-accent-hover">View all jobs</a>
  </div>

  <div class="px-4 py-3 border-b border-border-subtle flex items-center gap-4 text-xs">
    <span class="text-text-muted">Crawl ID</span>
    <span class="text-text-secondary font-mono">{crawl.crawl_id}</span>
  </div>

  <div class="p-4 space-y-1.5 max-h-[400px] overflow-y-auto">
    {#each crawl.job_ids as id, i}
      <a href="#{`jobs/${id}`}"
        class="flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-surface-raised transition-colors group">
        <span class="text-[11px] text-text-muted tabular-nums w-6">{i + 1}</span>
        <span class="text-xs text-text-secondary font-mono group-hover:text-accent transition-colors truncate">
          {id}
        </span>
        <svg class="w-3 h-3 text-text-muted ml-auto opacity-0 group-hover:opacity-100 transition-opacity shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
          <path d="M9 5l7 7-7 7" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </a>
    {/each}
  </div>
</div>
