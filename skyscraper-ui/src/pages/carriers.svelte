<script lang="ts">
  import Badge from '@components/ui/badge.svelte'
  import Button from '@components/ui/button.svelte'
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

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h1 class="text-xl font-bold text-text-primary">Carriers</h1>
    <button class="text-xs text-text-muted hover:text-text-secondary" onclick={load}>Refresh</button>
  </div>

  <div class="flex gap-1 border-b border-border-subtle">
    {#each [
      { id: 'carriers', label: 'Carriers' },
      { id: 'users', label: 'Proxy Users' },
      { id: 'sessions', label: 'Sessions' },
    ] as t}
      <button
        class="px-3 py-2 text-xs transition-colors border-b-2 -mb-px
          {tab === t.id ? 'border-accent text-accent' : 'border-transparent text-text-muted hover:text-text-secondary'}"
        onclick={() => (tab = t.id as typeof tab)}>{t.label}</button>
    {/each}
  </div>

  <div class="rounded-xl border border-border-subtle bg-surface overflow-hidden">
    {#if loading}
      <div class="p-8 text-center text-sm text-text-muted">Loading...</div>
    {:else if tab === 'carriers'}
      {#if carriers.length === 0}
        <div class="p-8 text-center text-sm text-text-muted">No carriers found</div>
      {:else}
        {#each carriers as c}
          <div class="flex items-center justify-between px-4 py-3 border-b border-border-subtle/50 last:border-0">
            <div>
              <p class="text-sm text-text-secondary">{c.name}</p>
              <p class="text-xs text-text-muted">{c.upstream_addr}</p>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs text-text-muted">{c.country}</span>
              <Badge text={c.online ? 'online' : 'offline'} color={c.online ? 'green' : 'red'} />
            </div>
          </div>
        {/each}
      {/if}
    {:else if tab === 'users'}
      {#if users.length === 0}
        <div class="p-8 text-center text-sm text-text-muted">No proxy users</div>
      {:else}
        <table class="w-full">
          <thead>
            <tr class="border-b border-border-subtle">
              {#each ['Email', 'Active', 'Actions'] as h}
                <th class="text-left text-[11px] text-text-muted uppercase tracking-wider px-4 py-2.5">{h}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each users as u}
              <tr class="border-b border-border-subtle/50">
                <td class="px-4 py-2.5 text-sm text-text-secondary">{u.email}</td>
                <td class="px-4 py-2.5"><Badge text={u.active ? 'active' : 'inactive'} color={u.active ? 'green' : 'red'} /></td>
                <td class="px-4 py-2.5 space-x-2">
                  <button class="text-xs text-text-muted hover:text-text-secondary" onclick={() => handleToggle(u.id)}>Toggle</button>
                  <button class="text-xs text-danger hover:underline" onclick={() => handleDelete(u.id)}>Delete</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else}
      {#if sessions.length === 0}
        <div class="p-8 text-center text-sm text-text-muted">No active sessions</div>
      {:else}
        <table class="w-full">
          <thead>
            <tr class="border-b border-border-subtle">
              {#each ['Session', 'Carrier', 'Up', 'Down', 'Active'] as h}
                <th class="text-left text-[11px] text-text-muted uppercase tracking-wider px-4 py-2.5">{h}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each sessions as s}
              <tr class="border-b border-border-subtle/50">
                <td class="px-4 py-2.5 text-xs text-text-muted font-mono">{s.id.slice(0, 8)}</td>
                <td class="px-4 py-2.5 text-sm text-text-secondary">{s.carrier_id.slice(0, 8)}</td>
                <td class="px-4 py-2.5 text-sm text-text-muted tabular-nums">{s.bytes_up}</td>
                <td class="px-4 py-2.5 text-sm text-text-muted tabular-nums">{s.bytes_down}</td>
                <td class="px-4 py-2.5"><Badge text={s.active ? 'active' : 'closed'} color={s.active ? 'green' : 'gray'} /></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {/if}
  </div>
</div>
