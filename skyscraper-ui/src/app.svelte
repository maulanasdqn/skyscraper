<script lang="ts">
  import AppShell from '@components/layout/app-shell.svelte'
  import Dashboard from '@pages/dashboard.svelte'
  import Jobs from '@pages/jobs.svelte'
  import JobDetail from '@pages/job-detail.svelte'
  import Scrape from '@pages/scrape.svelte'
  import Tenants from '@pages/tenants.svelte'
  import Carriers from '@pages/carriers.svelte'
  import Profiles from '@pages/profiles.svelte'
  import MetricsPage from '@pages/metrics-page.svelte'
  import Settings from '@pages/settings.svelte'

  let currentRoute = $state('playground')
  let routeParam = $state('')

  function parseHash() {
    const hash = window.location.hash.replace(/^#\/?/, '') || 'playground'
    const parts = hash.split('/')
    if (parts[0] === 'jobs' && parts[1]) {
      currentRoute = 'jobs-detail'
      routeParam = parts[1]
    } else {
      currentRoute = parts[0]
      routeParam = ''
    }
  }

  $effect(() => {
    parseHash()
    const handler = () => parseHash()
    window.addEventListener('hashchange', handler)
    return () => window.removeEventListener('hashchange', handler)
  })

  let shellRoute = $derived(
    currentRoute === 'jobs-detail' ? 'jobs' : currentRoute
  )
</script>

<AppShell bind:currentRoute={shellRoute}>
  {#if currentRoute === 'playground'}
    <Scrape />
  {:else if currentRoute === 'dashboard'}
    <Dashboard />
  {:else if currentRoute === 'jobs'}
    <Jobs />
  {:else if currentRoute === 'jobs-detail'}
    <JobDetail jobId={routeParam} />
  {:else if currentRoute === 'tenants'}
    <Tenants />
  {:else if currentRoute === 'carriers'}
    <Carriers />
  {:else if currentRoute === 'profiles'}
    <Profiles />
  {:else if currentRoute === 'metrics'}
    <MetricsPage />
  {:else if currentRoute === 'settings'}
    <Settings />
  {:else}
    <div class="text-center py-20">
      <p class="text-text-muted text-sm">Page not found</p>
      <a href="#playground" class="text-accent hover:underline text-sm">Go to Playground</a>
    </div>
  {/if}
</AppShell>
