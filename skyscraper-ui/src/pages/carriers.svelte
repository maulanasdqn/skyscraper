<script lang="ts">
  import Badge from '@components/ui/badge.svelte'
  import Button from '@components/ui/button.svelte'
  import EmptyState from '@components/ui/empty-state.svelte'
  import { getAdminKey, getParallaxUrl } from '@lib/stores/auth.svelte'
  import { listCarriers } from '@lib/api/skyscraper'
  import { listUsers, listSessions, toggleUser, deleteUser } from '@lib/api/parallax'
  import type { Carrier } from '@lib/types'
  import type { ParallaxUser, ParallaxSession } from '@lib/api/parallax'

  let carriers = $state<Carrier[]>([])
  let users = $state<ParallaxUser[]>([])
  let sessions = $state<ParallaxSession[]>([])
  let loading = $state(true)
  let tab = $state<'carriers' | 'users' | 'sessions'>('carriers')

  async function load() {
    loading = true
    const key = getAdminKey()
    const pUrl = getParallaxUrl()
    const [c, u, s] = await Promise.allSettled([
      key ? listCarriers(key) : Promise.reject(),
      listUsers(pUrl), listSessions(pUrl),
    ])
    if (c.status === 'fulfilled') carriers = c.value
    if (u.status === 'fulfilled') users = u.value
    if (s.status === 'fulfilled') sessions = s.value
    loading = false
  }

  async function handleToggle(id: string) {
    await toggleUser(getParallaxUrl(), id); load()
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this user?')) return
    await deleteUser(getParallaxUrl(), id); load()
  }

  $effect(() => { load() })
</script>

<div class="space-y-4 animate-fade-in">
  <div class="flex items-center justify-between">
    <div class="flex gap-1 bg-surface-overlay/50 rounded-lg p-0.5">
      {#each [
        { id: 'carriers', label: 'Carriers' },
        { id: 'users', label: 'Proxy Users' },
        { id: 'sessions', label: 'Sessions' },
      ] as t}
        <button
          class="px-3 py-1.5 text-xs rounded-md transition-all
            {tab === t.id ? 'bg-accent/15 text-accent font-medium' : 'text-text-muted hover:text-text-secondary'}"
          onclick={() => (tab = t.id as typeof tab)}>{t.label}</button>
      {/each}
    </div>
    <button class="text-xs text-text-muted hover:text-text-secondary transition-colors" onclick={load}>Refresh</button>
  </div>

  <div class="rounded-xl border border-border-subtle bg-surface overflow-hidden">
    {#if loading}
      <div class="p-8 text-center">
        <svg class="animate-spin h-5 w-5 mx-auto text-accent" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
      </div>
    {:else if tab === 'carriers'}
      {#if carriers.length === 0}
        <EmptyState icon="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          title="No carriers found" description="Configure carrier routes in Parallax to see them here" />
      {:else}
        {#each carriers as c}
          <div class="flex items-center justify-between px-5 py-3.5 border-b border-border-subtle/50 last:border-0 hover:bg-surface-raised/30 transition-colors">
            <div>
              <p class="text-sm text-text-primary font-medium">{c.name}</p>
              <p class="text-xs text-text-muted mt-0.5 font-mono">{c.upstream_addr}</p>
            </div>
            <div class="flex items-center gap-3">
              <span class="text-xs text-text-muted">{c.country}</span>
              <Badge text={c.online ? 'online' : 'offline'} color={c.online ? 'green' : 'red'} />
            </div>
          </div>
        {/each}
      {/if}
    {:else if tab === 'users'}
      {#if users.length === 0}
        <EmptyState icon="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197"
          title="No proxy users" description="Users are created through the Parallax API" />
      {:else}
        <table class="w-full">
          <thead>
            <tr class="border-b border-border-subtle">
              {#each ['Email', 'Active', 'Actions'] as h}
                <th class="text-left text-[11px] text-text-muted uppercase tracking-wider px-5 py-3">{h}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each users as u}
              <tr class="border-b border-border-subtle/50 hover:bg-surface-raised/30 transition-colors">
                <td class="px-5 py-3 text-sm text-text-secondary">{u.email}</td>
                <td class="px-5 py-3"><Badge text={u.active ? 'active' : 'inactive'} color={u.active ? 'green' : 'red'} /></td>
                <td class="px-5 py-3 space-x-3">
                  <button class="text-xs text-text-muted hover:text-text-secondary transition-colors" onclick={() => handleToggle(u.id)}>Toggle</button>
                  <button class="text-xs text-danger hover:text-danger/80 transition-colors" onclick={() => handleDelete(u.id)}>Delete</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else}
      {#if sessions.length === 0}
        <EmptyState icon="M13 10V3L4 14h7v7l9-11h-7z"
          title="No active sessions" description="Sessions appear when proxied connections are active" />
      {:else}
        <table class="w-full">
          <thead>
            <tr class="border-b border-border-subtle">
              {#each ['Session', 'Carrier', 'Up', 'Down', 'Active'] as h}
                <th class="text-left text-[11px] text-text-muted uppercase tracking-wider px-5 py-3">{h}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each sessions as s}
              <tr class="border-b border-border-subtle/50 hover:bg-surface-raised/30 transition-colors">
                <td class="px-5 py-3 text-xs text-text-muted font-mono">{s.id.slice(0, 8)}</td>
                <td class="px-5 py-3 text-sm text-text-secondary">{s.carrier_id.slice(0, 8)}</td>
                <td class="px-5 py-3 text-sm text-text-muted tabular-nums">{s.bytes_up}</td>
                <td class="px-5 py-3 text-sm text-text-muted tabular-nums">{s.bytes_down}</td>
                <td class="px-5 py-3"><Badge text={s.active ? 'active' : 'closed'} color={s.active ? 'green' : 'gray'} /></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {/if}
  </div>
</div>
