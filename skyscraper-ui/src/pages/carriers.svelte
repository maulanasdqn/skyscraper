<script lang="ts">
  import Badge from '@components/ui/Badge.svelte'
  import Button from '@components/ui/Button.svelte'
  import Card from '@components/ui/Card.svelte'
  import { getAdminKey, getParallaxUrl } from '@lib/stores/auth.svelte'
  import { listCarriers } from '@lib/api/skyscraper'
  import { getParallaxCarriers, listUsers, listSessions, toggleUser, deleteUser } from '@lib/api/parallax'
  import type { Carrier } from '@lib/types'
  import type { ParallaxUser, ParallaxSession } from '@lib/api/parallax'

  let carriers = $state<Carrier[]>([])
  let users = $state<ParallaxUser[]>([])
  let sessions = $state<ParallaxSession[]>([])
  let loading = $state(true)
  let tab = $state<'carriers' | 'users' | 'sessions'>('carriers')
  let error = $state('')

  async function loadCarriers() {
    const key = getAdminKey()
    if (!key) return
    try { carriers = await listCarriers(key) } catch { /* noop */ }
  }

  async function loadParallax() {
    const pUrl = getParallaxUrl()
    try {
      const [u, s] = await Promise.allSettled([listUsers(pUrl), listSessions(pUrl)])
      if (u.status === 'fulfilled') users = u.value
      if (s.status === 'fulfilled') sessions = s.value
    } catch { error = 'Cannot reach Parallax' }
  }

  async function load() {
    loading = true
    error = ''
    await Promise.all([loadCarriers(), loadParallax()])
    loading = false
  }

  async function handleToggle(id: string) {
    try { await toggleUser(getParallaxUrl(), id); loadParallax() }
    catch (e) { error = e instanceof Error ? e.message : 'Failed' }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this user?')) return
    try { await deleteUser(getParallaxUrl(), id); loadParallax() }
    catch (e) { error = e instanceof Error ? e.message : 'Failed' }
  }

  $effect(() => { load() })
</script>

<div class="space-y-4">
  <div class="flex gap-2">
    {#each [
      { id: 'carriers', label: 'Carriers' },
      { id: 'users', label: 'Proxy Users' },
      { id: 'sessions', label: 'Sessions' },
    ] as t}
      <button
        class="px-3 py-1.5 text-sm rounded-lg transition-colors
          {tab === t.id ? 'bg-indigo-600 text-white' : 'text-gray-400 hover:bg-gray-800'}"
        onclick={() => (tab = t.id as typeof tab)}>{t.label}</button>
    {/each}
    <Button size="sm" variant="ghost" onclick={load} class="ml-auto">Refresh</Button>
  </div>

  {#if error}
    <p class="text-sm text-red-400">{error}</p>
  {/if}

  {#if tab === 'carriers'}
    <Card title="Carriers (via Parallax)">
      {#if loading}
        <p class="text-sm text-gray-500 animate-pulse">Loading...</p>
      {:else if carriers.length === 0}
        <p class="text-sm text-gray-500">No carriers found</p>
      {:else}
        <div class="grid gap-3">
          {#each carriers as c}
            <div class="flex items-center justify-between p-3 rounded-lg bg-gray-800/50 border border-gray-800">
              <div>
                <p class="text-sm font-medium text-gray-200">{c.name}</p>
                <p class="text-xs text-gray-500">{c.upstream_addr}</p>
              </div>
              <div class="flex items-center gap-3">
                <span class="text-xs text-gray-400">{c.country}</span>
                <Badge text={c.online ? 'online' : 'offline'} color={c.online ? 'green' : 'red'} />
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </Card>
  {/if}

  {#if tab === 'users'}
    <Card title="Proxy Users">
      {#if users.length === 0}
        <p class="text-sm text-gray-500">No proxy users found. Connect Parallax in Settings.</p>
      {:else}
        <table class="w-full text-sm">
          <thead>
            <tr class="text-left text-gray-500 text-xs uppercase">
              <th class="pb-2">Email</th><th class="pb-2">API Key</th>
              <th class="pb-2">Active</th><th class="pb-2">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#each users as u}
              <tr class="hover:bg-gray-800/40">
                <td class="py-2 text-gray-300">{u.email}</td>
                <td class="py-2 text-gray-500 font-mono text-xs">{u.api_key.slice(0, 12)}…</td>
                <td class="py-2"><Badge text={u.active ? 'active' : 'inactive'}
                  color={u.active ? 'green' : 'red'} /></td>
                <td class="py-2 space-x-1">
                  <Button size="sm" variant="ghost" onclick={() => handleToggle(u.id)}>Toggle</Button>
                  <Button size="sm" variant="danger" onclick={() => handleDelete(u.id)}>Delete</Button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </Card>
  {/if}

  {#if tab === 'sessions'}
    <Card title="Active Sessions">
      {#if sessions.length === 0}
        <p class="text-sm text-gray-500">No active sessions</p>
      {:else}
        <table class="w-full text-sm">
          <thead>
            <tr class="text-left text-gray-500 text-xs uppercase">
              <th class="pb-2">Session</th><th class="pb-2">Carrier</th>
              <th class="pb-2">Up</th><th class="pb-2">Down</th><th class="pb-2">Active</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#each sessions as s}
              <tr>
                <td class="py-2 text-gray-400 font-mono text-xs">{s.id.slice(0, 8)}</td>
                <td class="py-2 text-gray-300">{s.carrier_id.slice(0, 8)}</td>
                <td class="py-2 text-gray-400">{s.bytes_up}</td>
                <td class="py-2 text-gray-400">{s.bytes_down}</td>
                <td class="py-2"><Badge text={s.active ? 'active' : 'closed'}
                  color={s.active ? 'green' : 'gray'} /></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </Card>
  {/if}
</div>
