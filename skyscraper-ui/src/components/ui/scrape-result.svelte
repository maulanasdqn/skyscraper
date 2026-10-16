<script lang="ts">
  import { formatMs, formatBytes } from '@lib/utils/format'
  import { downloadJson, downloadCsv } from '@lib/utils/export'
  import type { ScrapeResult } from '@lib/types'

  let {
    scrape,
    jobId = '',
    aiResult = null,
    aiLoading = false,
    wallTimeMs = 0,
  }: {
    scrape: ScrapeResult
    jobId?: string
    aiResult?: { data: Record<string, unknown>; model: string } | null
    aiLoading?: boolean
    wallTimeMs?: number
  } = $props()

  let tab = $state<'ai' | 'meta' | 'extracted' | 'html' | 'render'>('meta')
  let copied = $state(false)
  let exportOpen = $state(false)

  let hasExtracted = $derived(scrape.extracted && Object.keys(scrape.extracted).length > 0)
  let displayTime = $derived(wallTimeMs > 0 ? formatMs(wallTimeMs) : formatMs(scrape.elapsed_ms))
  let isBlocked = $derived(scrape.blocked || scrape.html_length < 10000)

  $effect(() => {
    if (aiResult) tab = 'ai'
    else if (hasExtracted) tab = 'extracted'
    else tab = 'meta'
  })

  function hl(obj: unknown): string {
    return JSON.stringify(obj, null, 2)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/"([^"]+)":/g, '<span class="text-accent">"$1"</span>:')
      .replace(/: "([^"]*)"/g, ': <span class="text-success">"$1"</span>')
      .replace(/: (\d+\.?\d*)/g, ': <span class="text-warning">$1</span>')
      .replace(/: (true|false)/g, ': <span class="text-info">$1</span>')
      .replace(/: (null)/g, ': <span class="text-text-muted">$1</span>')
  }

  function hlHtml(raw: string): string {
    const t = raw.length > 50000 ? raw.slice(0, 50000) + '\n...' : raw
    return t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
      .replace(/&lt;(\/?)([\w-]+)/g, '&lt;$1<span class="text-accent">$2</span>')
      .replace(/ ([\w-]+)="/g, ' <span class="text-warning">$1</span>="')
  }

  async function copyResult() {
    const data = tab === 'ai' ? aiResult?.data : tab === 'extracted' ? scrape.extracted : tab === 'html' ? scrape.html : null
    if (!data) return
    await navigator.clipboard.writeText(typeof data === 'string' ? data : JSON.stringify(data, null, 2))
    copied = true
    setTimeout(() => (copied = false), 1500)
  }

  function exportAs(format: 'json' | 'csv') {
    const data = aiResult?.data ?? scrape.extracted ?? {}
    const name = new URL(scrape.final_url || 'result').hostname.replace(/\./g, '-') || 'result'
    if (format === 'json') downloadJson(data, `${name}.json`)
    else downloadCsv(data as Record<string, unknown>, `${name}.csv`)
    exportOpen = false
  }
</script>

<div class="rounded-xl border {isBlocked ? 'border-warning/30' : 'border-border-subtle'} bg-surface overflow-hidden">
  <div class="flex items-center justify-between px-4 py-2.5 border-b border-border-subtle">
    <div class="flex items-center gap-3 text-sm">
      <span class="w-1.5 h-1.5 rounded-full {isBlocked ? 'bg-warning' : 'bg-success'}"></span>
      <span class="text-text-primary font-semibold tabular-nums">{displayTime}</span>
      <span class="text-text-muted text-xs">{formatBytes(scrape.html_length)}</span>
      {#if isBlocked}<span class="text-warning text-xs">blocked</span>{/if}
    </div>
    <div class="flex items-center gap-2">
      {#if tab !== 'meta' && tab !== 'render'}
        <button onclick={copyResult}
          class="text-[11px] text-text-muted hover:text-text-secondary transition-colors flex items-center gap-1">
          {#if copied}
            <svg class="w-3 h-3 text-success" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path d="M5 13l4 4L19 7" stroke-linecap="round" stroke-linejoin="round"/></svg>
            Copied
          {:else}
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2"><path d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10" stroke-linecap="round" stroke-linejoin="round"/></svg>
            Copy
          {/if}
        </button>
      {/if}
      <div class="relative">
        <button onclick={() => (exportOpen = !exportOpen)}
          class="text-[11px] text-text-muted hover:text-text-secondary transition-colors flex items-center gap-1">
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
            <path d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          Export
        </button>
        {#if exportOpen}
          <div class="absolute right-0 top-full mt-1 bg-surface-overlay border border-border-subtle rounded-lg shadow-lg z-50 py-1 min-w-[100px]">
            <button onclick={() => exportAs('json')}
              class="w-full text-left px-3 py-1.5 text-[11px] text-text-secondary hover:bg-surface-raised transition-colors">JSON</button>
            <button onclick={() => exportAs('csv')}
              class="w-full text-left px-3 py-1.5 text-[11px] text-text-secondary hover:bg-surface-raised transition-colors">CSV</button>
          </div>
        {/if}
      </div>
      {#if jobId}
        <a href="#{`jobs/${jobId}`}" class="text-[11px] text-accent hover:text-accent-hover">View job</a>
      {/if}
    </div>
  </div>

  {#if isBlocked}
    <div class="px-4 py-2 bg-warning/5 border-b border-warning/10 text-xs text-warning">
      Page blocked (captcha/bot detection). Try with a proxy or different profile.
    </div>
  {/if}

  <div class="flex border-b border-border-subtle overflow-x-auto">
    {#each [
      ...(aiResult || aiLoading ? [{ id: 'ai', label: 'AI Extract' }] : []),
      { id: 'meta', label: 'Metadata' },
      ...(hasExtracted ? [{ id: 'extracted', label: 'CSS Data' }] : []),
      ...(scrape.html ? [{ id: 'html', label: 'HTML' }, { id: 'render', label: 'Preview' }] : []),
    ] as t}
      <button
        class="px-3.5 py-2 text-[11px] border-b-2 -mb-px transition-colors whitespace-nowrap
          {tab === t.id ? 'border-accent text-accent font-medium' : 'border-transparent text-text-muted hover:text-text-secondary'}"
        onclick={() => (tab = t.id as typeof tab)}>{t.label}</button>
    {/each}
  </div>

  <div class="relative min-h-[120px]">
    {#if tab === 'ai'}
      {#if aiLoading}
        <div class="p-6 flex items-center gap-3">
          <svg class="animate-spin h-4 w-4 text-accent shrink-0" viewBox="0 0 24 24" fill="none">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
          <span class="text-xs text-text-muted">Processing with Netra AI...</span>
        </div>
      {:else if aiResult}
        <div class="absolute top-2.5 right-3 flex items-center gap-2 z-10">
          {#if aiResult.model}<span class="text-[10px] text-text-muted bg-surface/80 px-1.5 py-0.5 rounded">{aiResult.model}</span>{/if}
        </div>
        <pre class="p-4 text-xs overflow-auto font-mono leading-relaxed max-h-[500px]">{@html hl(aiResult.data)}</pre>
      {/if}
    {:else if tab === 'meta'}
      <div class="p-4 grid grid-cols-2 gap-x-8 gap-y-2 text-sm">
        {#each [
          ['URL', scrape.final_url],
          ['Size', formatBytes(scrape.html_length)],
          ['Time', displayTime],
          ['Attempts', scrape.attempts],
          ['Profile', scrape.profile_used || '—'],
          ['Proxy', scrape.proxy_used || 'direct'],
        ] as [k, v]}
          <div class="flex justify-between py-1">
            <span class="text-text-muted">{k}</span>
            <span class="text-text-secondary text-right break-all max-w-[70%]">{v}</span>
          </div>
        {/each}
      </div>
    {:else if tab === 'extracted'}
      <pre class="p-4 text-xs overflow-auto font-mono leading-relaxed max-h-[500px]">{@html hl(scrape.extracted)}</pre>
    {:else if tab === 'html'}
      <pre class="p-4 text-xs overflow-auto font-mono leading-relaxed max-h-[500px]">{@html hlHtml(scrape.html ?? '')}</pre>
    {:else if tab === 'render'}
      <iframe
        srcdoc={scrape.html ?? ''}
        sandbox="allow-same-origin"
        class="w-full h-[500px] bg-white border-0"
        title="Page preview"
      ></iframe>
    {/if}
  </div>
</div>
