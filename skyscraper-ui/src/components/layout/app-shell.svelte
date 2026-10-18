<script lang="ts">
  import type { Snippet } from 'svelte'
  import Sidebar from './sidebar.svelte'
  import Toast from '@components/ui/toast.svelte'

  let {
    currentRoute = $bindable('playground'),
    children,
  }: {
    currentRoute?: string
    children?: Snippet
  } = $props()

  const pageTitles: Record<string, { title: string; desc: string }> = {
    playground: { title: 'Playground', desc: 'Scrape any URL with anti-detection browser' },
    dashboard: { title: 'Overview', desc: 'System health and recent activity' },
    jobs: { title: 'Jobs', desc: 'Track and manage scraping jobs' },
    tenants: { title: 'Tenants', desc: 'API key management and rate limits' },
    carriers: { title: 'Carriers', desc: 'Proxy carriers and sessions' },
    profiles: { title: 'Profiles', desc: 'Chromium fingerprint profiles' },
    metrics: { title: 'Metrics', desc: 'Performance and queue statistics' },
    settings: { title: 'Settings', desc: 'API keys and service configuration' },
  }

  let info = $derived(pageTitles[currentRoute] ?? { title: '', desc: '' })
</script>

<div class="flex h-screen bg-[#09090f] text-text-primary">
  <Sidebar bind:currentRoute />
  <div class="flex-1 flex flex-col min-w-0">
    <header class="h-14 flex items-center justify-between px-8 border-b border-border-subtle shrink-0 bg-surface-sunken/50">
      <div class="flex items-center gap-3 min-w-0">
        <h1 class="text-[15px] font-semibold text-text-primary truncate">{info.title}</h1>
        <span class="hidden sm:block text-xs text-text-muted truncate">{info.desc}</span>
      </div>
      <div class="flex items-center gap-3">
        <div class="w-2 h-2 rounded-full bg-success" title="Online"></div>
        <span class="text-[11px] text-text-muted">v0.1.0</span>
      </div>
    </header>
    <main class="flex-1 overflow-y-auto">
      <div class="max-w-6xl mx-auto px-6 lg:px-8 py-6">
        {#if children}{@render children()}{/if}
      </div>
    </main>
  </div>
</div>
<Toast />
