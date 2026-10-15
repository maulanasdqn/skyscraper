<script lang="ts">
  let {
    currentRoute = $bindable('dashboard'),
  }: {
    currentRoute?: string
  } = $props()

  let mobileOpen = $state(false)

  type NavItem = { id: string; icon: string; label: string }

  const navItems: NavItem[] = [
    { id: 'dashboard', icon: '◫', label: 'Dashboard' },
    { id: 'jobs', icon: '⋯', label: 'Jobs' },
    { id: 'scrape', icon: '⌗', label: 'Scrape' },
    { id: 'tenants', icon: '⊕', label: 'Tenants' },
    { id: 'carriers', icon: '⇄', label: 'Carriers' },
    { id: 'profiles', icon: '⊡', label: 'Profiles' },
    { id: 'metrics', icon: '⊞', label: 'Metrics' },
    { id: 'settings', icon: '⊙', label: 'Settings' },
  ]

  function navigate(id: string) {
    currentRoute = id
    window.location.hash = id
    mobileOpen = false
  }
</script>

<button
  class="lg:hidden fixed top-4 left-4 z-50 p-2 rounded-lg bg-gray-800 text-gray-300"
  onclick={() => (mobileOpen = !mobileOpen)}
>
  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" />
  </svg>
</button>

{#if mobileOpen}
  <div
    class="lg:hidden fixed inset-0 bg-black/50 z-30"
    role="button"
    tabindex="-1"
    onclick={() => (mobileOpen = false)}
    onkeydown={() => {}}
  ></div>
{/if}

<aside class="
  fixed lg:static inset-y-0 left-0 z-40
  w-56 bg-gray-950 border-r border-gray-800
  flex flex-col
  transform transition-transform duration-200
  {mobileOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'}
">
  <div class="h-16 flex items-center gap-2 px-5 border-b border-gray-800">
    <span class="text-lg font-bold text-indigo-400">⬡</span>
    <span class="text-lg font-bold text-gray-100">SkyScraper</span>
  </div>

  <nav class="flex-1 py-4 px-3 space-y-1 overflow-y-auto">
    {#each navItems as item}
      <button
        onclick={() => navigate(item.id)}
        class="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors
          {currentRoute === item.id
            ? 'bg-indigo-600/20 text-indigo-400'
            : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800/50'}"
      >
        <span class="text-base w-5 text-center">{item.icon}</span>
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>

  <div class="p-4 border-t border-gray-800">
    <p class="text-xs text-gray-600">SkyScraper v0.1.0</p>
  </div>
</aside>
