<script lang="ts">
  let {
    currentRoute = $bindable('playground'),
  }: {
    currentRoute?: string
  } = $props()

  let mobileOpen = $state(false)

  type NavItem = { id: string; icon: string; label: string; separator?: boolean }

  const navItems: NavItem[] = [
    { id: 'playground', icon: 'M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z', label: 'Playground' },
    { id: 'dashboard', icon: 'M4 5a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1H5a1 1 0 01-1-1V5zm10 0a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1V5zM4 15a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1H5a1 1 0 01-1-1v-4zm10 0a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z', label: 'Overview' },
    { id: 'jobs', icon: 'M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 012-2h2a2 2 0 012 2M9 5h6', label: 'Jobs' },
    { id: 'sep1', icon: '', label: '', separator: true },
    { id: 'tenants', icon: 'M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197', label: 'Tenants' },
    { id: 'carriers', icon: 'M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z', label: 'Carriers' },
    { id: 'profiles', icon: 'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z', label: 'Profiles' },
    { id: 'metrics', icon: 'M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z', label: 'Metrics' },
    { id: 'sep2', icon: '', label: '', separator: true },
    { id: 'settings', icon: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z', label: 'Settings' },
  ]

  function navigate(id: string) {
    currentRoute = id
    window.location.hash = id
    mobileOpen = false
  }
</script>

<button
  class="lg:hidden fixed top-4 left-4 z-50 p-2 rounded-lg bg-surface-raised text-text-secondary"
  aria-label="Menu"
  onclick={() => (mobileOpen = !mobileOpen)}
>
  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 6h16M4 12h16M4 18h16" />
  </svg>
</button>

{#if mobileOpen}
  <div class="lg:hidden fixed inset-0 bg-black/60 z-30"
    role="button" tabindex="-1"
    onclick={() => (mobileOpen = false)} onkeydown={() => {}}></div>
{/if}

<aside class="
  fixed lg:static inset-y-0 left-0 z-40
  w-16 bg-surface border-r border-border-subtle
  flex flex-col items-center
  transform transition-transform duration-200
  {mobileOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'}
">
  <div class="h-14 flex items-center justify-center border-b border-border-subtle w-full">
    <a href="#playground" class="w-8 h-8 rounded-xl bg-accent/20 flex items-center justify-center hover:bg-accent/30 transition-colors">
      <svg class="w-4 h-4 text-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M4 20 L12 4 L20 20" stroke-linecap="round" stroke-linejoin="round" />
        <path d="M8 14 L16 14" stroke-linecap="round" />
      </svg>
    </a>
  </div>

  <nav class="flex-1 py-3 flex flex-col items-center gap-0.5 overflow-y-auto w-full px-2">
    {#each navItems as item}
      {#if item.separator}
        <div class="w-6 h-px bg-border-subtle my-2"></div>
      {:else}
        <div class="relative group">
          <button
            onclick={() => navigate(item.id)}
            aria-label={item.label}
            class="w-10 h-10 flex items-center justify-center rounded-xl transition-all
              {currentRoute === item.id
                ? 'bg-accent/15 text-accent'
                : 'text-text-muted hover:text-text-secondary hover:bg-surface-raised'}"
          >
            <svg class="w-[18px] h-[18px]" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d={item.icon} />
            </svg>
          </button>
          <div class="absolute left-full ml-2 top-1/2 -translate-y-1/2 px-2 py-1 bg-surface-overlay text-text-primary text-[11px] rounded-md
            opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity whitespace-nowrap z-50 shadow-lg border border-border-subtle">
            {item.label}
          </div>
        </div>
      {/if}
    {/each}
  </nav>

  <div class="py-3 border-t border-border-subtle w-full flex justify-center">
    <div class="w-2 h-2 rounded-full bg-success" title="Online"></div>
  </div>
</aside>
