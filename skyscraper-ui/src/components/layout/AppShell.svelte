<script lang="ts">
  import type { Snippet } from 'svelte'
  import Sidebar from './Sidebar.svelte'

  let {
    currentRoute = $bindable('dashboard'),
    children,
  }: {
    currentRoute?: string
    children?: Snippet
  } = $props()

  const routeTitles: Record<string, string> = {
    dashboard: 'Dashboard',
    jobs: 'Jobs',
    scrape: 'Scrape',
    tenants: 'Tenants',
    carriers: 'Carriers',
    profiles: 'Profiles',
    metrics: 'Metrics',
    settings: 'Settings',
  }
</script>

<div class="flex h-screen bg-gray-950 text-gray-100">
  <Sidebar bind:currentRoute />

  <div class="flex-1 flex flex-col min-w-0 lg:ml-0">
    <header class="h-16 flex items-center px-6 border-b border-gray-800 bg-gray-950/80 backdrop-blur-sm sticky top-0 z-10">
      <div class="pl-10 lg:pl-0">
        <h1 class="text-lg font-semibold text-gray-100">
          {routeTitles[currentRoute] ?? currentRoute}
        </h1>
      </div>
    </header>

    <main class="flex-1 overflow-y-auto p-6">
      {#if children}{@render children()}{/if}
    </main>
  </div>
</div>
