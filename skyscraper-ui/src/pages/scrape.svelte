<script lang="ts">
  import Button from '@components/ui/button.svelte'
  import ScrapeResult from '@components/ui/scrape-result.svelte'
  import BatchResult from '@components/ui/batch-result.svelte'
  import CrawlResult from '@components/ui/crawl-result.svelte'
  import LoadingCard from '@components/ui/loading-card.svelte'
  import { ensureTenant, getCosmiumApiKey } from '@lib/stores/auth.svelte'
  import { submitJob, scrapeSync, submitBatch, submitCrawl, type SubmitJobData, type CrawlData } from '@lib/api/skyscraper'
  import { aiFormat } from '@lib/api/cosmium'
  import { extractStructuredData, extractPageText } from '@lib/utils/html-cleaner'
  import type { ScrapeResult as ScrapeResultType, CrawlResponse } from '@lib/types'

  let url = $state(''), batchUrls = $state(''), aiPrompt = $state(''), extractors = $state('')
  let waitMs = $state('1500'), maxPages = $state('10'), maxDepth = $state('2')
  let screenshot = $state(false), showAdvanced = $state(false)
  let syncMode = $state<'sync' | 'async'>('sync')
  let extractMode = $state<'ai' | 'css'>('ai')
  let playMode = $state<'single' | 'batch' | 'crawl'>('single')
  let phase = $state<'idle' | 'scraping' | 'extracting' | 'done' | 'error'>('idle')
  let scrape = $state<ScrapeResultType | null>(null), error = $state(''), jobId = $state('')
  let aiResult = $state<{ data: Record<string, unknown>; model: string } | null>(null)
  let wallTimeMs = $state(0), batchJobIds = $state<string[]>([]), crawlResult = $state<CrawlResponse | null>(null)

  const inputCls = 'rounded-xl border border-border-subtle bg-surface-raised text-sm text-text-primary placeholder-text-muted focus:outline-none focus:ring-1 focus:ring-accent/40 focus:border-accent/40'

  function buildReq(u: string): SubmitJobData {
    return { url: u, extract: extractMode === 'css' && extractors ? extractors.split(',').map(s => s.trim()).filter(Boolean) : [],
      wait_ms: parseInt(waitMs) || 1500, screenshot, include_html: true }
  }

  function reset() { phase = 'idle'; scrape = null; error = ''; aiResult = null; jobId = ''; wallTimeMs = 0; batchJobIds = []; crawlResult = null }

  async function getKey() { try { return await ensureTenant() } catch { error = 'Failed to provision'; phase = 'error'; return null } }

  async function handleSingle() {
    if (!url) return; reset(); phase = 'scraping'
    const key = await getKey(); if (!key) return
    const t0 = performance.now()
    try {
      if (syncMode === 'async') { const j = await submitJob(key, buildReq(url)); window.location.hash = `jobs/${j.id}`; return }
      const res = await scrapeSync(key, buildReq(url), 90000)
      wallTimeMs = Math.round(performance.now() - t0); scrape = res.result ?? null; jobId = res.job_id
      if (extractMode === 'ai' && scrape?.html) {
        const ld = extractStructuredData(scrape.html)
        if (ld && !aiPrompt) { aiResult = { data: ld, model: 'json-ld (instant)' } }
        else {
          phase = 'extracting'
          const p = aiPrompt || 'Extract: product name, price, currency, rating, review count, availability, description, images. Omit fields without values.'
          try { aiResult = await aiFormat('', url, p, { content: extractPageText(scrape.html) }, getCosmiumApiKey()) }
          catch (e) { aiResult = { data: { error: e instanceof Error ? e.message : 'AI failed' }, model: '' } }
        }
      }
      phase = 'done'
    } catch (e) { error = e instanceof Error ? e.message : 'Scrape failed'; phase = 'error' }
  }

  async function handleBatch() {
    const urls = batchUrls.split('\n').map(u => u.trim()).filter(Boolean)
    if (!urls.length) return; if (urls.length > 100) { error = 'Max 100 URLs per batch'; phase = 'error'; return }
    reset(); phase = 'scraping'
    const key = await getKey(); if (!key) return
    const t0 = performance.now()
    try { const r = await submitBatch(key, urls.map(u => buildReq(u))); wallTimeMs = Math.round(performance.now() - t0); batchJobIds = r.job_ids; phase = 'done' }
    catch (e) { error = e instanceof Error ? e.message : 'Batch failed'; phase = 'error' }
  }

  async function handleCrawl() {
    if (!url) return; reset(); phase = 'scraping'
    const key = await getKey(); if (!key) return
    const t0 = performance.now()
    try { crawlResult = await submitCrawl(key, { start_url: url, max_pages: parseInt(maxPages) || 10, max_depth: parseInt(maxDepth) || 2, include_html: true, screenshot }); wallTimeMs = Math.round(performance.now() - t0); phase = 'done' }
    catch (e) { error = e instanceof Error ? e.message : 'Crawl failed'; phase = 'error' }
  }

  function submit() { if (playMode === 'single') handleSingle(); else if (playMode === 'batch') handleBatch(); else handleCrawl() }
  let urlCount = $derived(batchUrls.split('\n').filter(l => l.trim()).length)
  let busy = $derived(phase === 'scraping' || phase === 'extracting')
  let canSubmit = $derived(!busy && (playMode === 'batch' ? urlCount > 0 : !!url))
  let btnText = $derived(busy ? (phase === 'extracting' ? 'Extracting...' : playMode === 'batch' ? 'Submitting...' : playMode === 'crawl' ? 'Crawling...' : 'Scraping...') : playMode === 'batch' ? `Submit ${urlCount || ''}` : playMode === 'crawl' ? 'Crawl' : 'Scrape')
  function onKey(e: KeyboardEvent) { if (e.key === 'Enter' && canSubmit) submit() }
</script>

<div class="space-y-5">
  <div>
    <h1 class="text-xl font-bold text-text-primary">Playground</h1>
    <p class="text-sm text-text-muted mt-1">Scrape any URL with anti-detection browser</p>
  </div>

  <div class="rounded-xl border border-border-subtle bg-surface">
    <div class="flex items-center gap-2 px-4 pt-3">
      <div class="flex gap-0.5 bg-surface-overlay/50 rounded-lg p-0.5">
        {#each [{ id: 'single', label: 'Single' }, { id: 'batch', label: 'Batch' }, { id: 'crawl', label: 'Crawl' }] as m}
          <button class="px-3 py-1.5 text-[11px] rounded-md transition-all
            {playMode === m.id ? 'bg-accent/15 text-accent font-medium' : 'text-text-muted hover:text-text-secondary'}"
            onclick={() => { playMode = m.id as typeof playMode; reset() }}>{m.label}</button>
        {/each}
      </div>
    </div>

    <div class="p-4 space-y-3">
      {#if playMode === 'batch'}
        <div class="flex gap-2 items-end">
          <div class="flex-1 relative">
            <span class="absolute left-3 top-2 text-[10px] text-text-muted uppercase tracking-wider">URLs (one per line)</span>
            <textarea bind:value={batchUrls} rows={5} placeholder={"https://example.com\nhttps://example.org"}
              class="w-full {inputCls} pl-3 pr-3 pt-7 pb-2 resize-none font-mono"></textarea>
            {#if urlCount > 0}<span class="absolute right-3 top-2 text-[10px] text-text-muted">{urlCount}/100</span>{/if}
          </div>
          <Button variant="primary" size="lg" onclick={submit} loading={busy} disabled={!canSubmit}>{btnText}</Button>
        </div>
      {:else}
        <div class="flex gap-2">
          <input type="text" bind:value={url} onkeydown={onKey}
            placeholder={playMode === 'crawl' ? 'Enter start URL to crawl...' : 'Enter URL to scrape...'}
            class="flex-1 {inputCls} pl-4 pr-4 py-3" />
          <Button variant="primary" size="lg" onclick={submit} loading={busy} disabled={!canSubmit}>{btnText}</Button>
        </div>
      {/if}

      {#if playMode === 'single' && extractMode === 'ai'}
        <div class="relative">
          <span class="absolute left-3 top-2.5 text-[10px] text-text-muted uppercase tracking-wider">AI Prompt</span>
          <textarea bind:value={aiPrompt} rows={1} placeholder="e.g. Extract product name, price, rating  (leave blank for auto)"
            class="w-full {inputCls} pl-3 pr-3 pt-7 pb-2 resize-none"></textarea>
        </div>
      {:else if playMode === 'single' && extractMode === 'css'}
        <div class="relative">
          <span class="absolute left-3 top-2.5 text-[10px] text-text-muted uppercase tracking-wider">CSS Selectors</span>
          <input bind:value={extractors} placeholder="e.g. h1, .price, #description" class="w-full {inputCls} pl-3 pr-3 pt-7 pb-2" />
        </div>
      {/if}

      {#if playMode === 'crawl'}
        <div class="flex gap-3">
          {#each [{ v: maxPages, label: 'Max Pages', min: 1, max: 100 }, { v: maxDepth, label: 'Max Depth', min: 0, max: 5 }] as f}
            <div class="flex flex-col gap-1">
              <label class="text-[10px] text-text-muted uppercase tracking-wider">{f.label}</label>
              <input type="number" value={f.v} min={f.min} max={f.max}
                oninput={(e) => { if (f.label === 'Max Pages') maxPages = (e.target as HTMLInputElement).value; else maxDepth = (e.target as HTMLInputElement).value }}
                class="w-20 rounded-lg border border-border-subtle bg-surface-raised px-2.5 py-1.5 text-xs text-text-primary focus:outline-none focus:ring-1 focus:ring-accent/40" />
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="flex items-center justify-between px-4 py-2.5 border-t border-border-subtle bg-surface-raised/30">
      <div class="flex items-center gap-3">
        {#if playMode === 'single'}
          <div class="flex gap-0.5 bg-surface-overlay/50 rounded-lg p-0.5">
            {#each [{ id: 'ai', label: 'AI' }, { id: 'css', label: 'CSS' }] as m}
              <button class="px-2.5 py-1 text-[11px] rounded-md transition-all
                {extractMode === m.id ? 'bg-accent/15 text-accent font-medium' : 'text-text-muted hover:text-text-secondary'}"
                onclick={() => (extractMode = m.id as 'ai' | 'css')}>{m.label}</button>
            {/each}
          </div>
          <div class="flex gap-0.5 bg-surface-overlay/50 rounded-lg p-0.5">
            {#each [{ id: 'sync', label: 'Sync' }, { id: 'async', label: 'Async' }] as m}
              <button class="px-2.5 py-1 text-[11px] rounded-md transition-all
                {syncMode === m.id ? 'bg-accent/15 text-accent font-medium' : 'text-text-muted hover:text-text-secondary'}"
                onclick={() => (syncMode = m.id as 'sync' | 'async')}>{m.label}</button>
            {/each}
          </div>
        {/if}
      </div>
      <button class="text-[11px] text-text-muted hover:text-text-secondary transition-colors flex items-center gap-1"
        onclick={() => (showAdvanced = !showAdvanced)}>
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
          <path d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        Advanced
      </button>
    </div>

    {#if showAdvanced}
      <div class="px-4 py-3 border-t border-border-subtle flex items-center gap-6">
        <div class="flex flex-col gap-1">
          <label class="text-[10px] text-text-muted uppercase tracking-wider">Wait (ms)</label>
          <input type="number" bind:value={waitMs} class="w-20 rounded-lg border border-border-subtle bg-surface-raised px-2.5 py-1.5 text-xs text-text-primary focus:outline-none focus:ring-1 focus:ring-accent/40" />
        </div>
        <label class="flex items-center gap-2 text-xs text-text-secondary cursor-pointer mt-4">
          <input type="checkbox" checked={screenshot} onchange={() => screenshot = !screenshot} class="rounded border-border-default bg-surface-raised accent-accent" />
          Screenshot
        </label>
      </div>
    {/if}
  </div>

  {#if busy}
    <LoadingCard
      title={playMode === 'batch' ? 'Submitting batch...' : playMode === 'crawl' ? 'Crawling pages...' : phase === 'scraping' ? 'Scraping page...' : 'AI extracting data...'}
      subtitle={playMode === 'batch' ? `Queuing ${urlCount} URLs` : playMode === 'crawl' ? 'Discovering and queueing pages' : phase === 'scraping' ? 'Loading with anti-detection browser' : 'Processing with Netra AI'} />
  {/if}

  {#if phase === 'error'}<div class="rounded-xl border border-danger/20 bg-danger/5 p-4 text-sm text-danger">{error}</div>{/if}

  {#if phase === 'done' && playMode === 'batch' && batchJobIds.length > 0}<BatchResult jobIds={batchJobIds} {wallTimeMs} />{/if}
  {#if phase === 'done' && playMode === 'crawl' && crawlResult}<CrawlResult crawl={crawlResult} {wallTimeMs} />{/if}
  {#if scrape && (phase === 'done' || phase === 'extracting') && playMode === 'single'}<ScrapeResult {scrape} {jobId} {aiResult} aiLoading={phase === 'extracting'} {wallTimeMs} />{/if}
</div>
