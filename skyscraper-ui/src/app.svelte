<script lang="ts">
  import AppShell from '@components/layout/AppShell.svelte'
  import Dashboard from '@pages/Dashboard.svelte'
  import Jobs from '@pages/Jobs.svelte'
  import JobDetail from '@pages/JobDetail.svelte'
  import Scrape from '@pages/Scrape.svelte'
  import Tenants from '@pages/Tenants.svelte'
  import Carriers from '@pages/Carriers.svelte'
  import Profiles from '@pages/Profiles.svelte'
  import MetricsPage from '@pages/MetricsPage.svelte'
  import Settings from '@pages/Settings.svelte'

  let currentRoute = $state('dashboard')
  let routeParam = $state('')

  function parseHash() {
    const hash = window.location.hash.replace(/^#\/?/, '') || 'dashboard'
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
  {#if currentRoute === 'dashboard'}
    <Dashboard />
  {:else if currentRoute === 'jobs'}
    <Jobs />
  {:else if currentRoute === 'jobs-detail'}
    <JobDetail jobId={routeParam} />
  {:else if currentRoute === 'scrape'}
    <Scrape />
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
      <p class="text-gray-500">Page not found</p>
      <a href="#dashboard" class="text-indigo-400 hover:underline text-sm">Go to Dashboard</a>
    </div>
  {/if}
</AppShell>
