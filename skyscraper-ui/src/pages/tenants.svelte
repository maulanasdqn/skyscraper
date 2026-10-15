<script lang="ts">
  import Badge from '@components/ui/Badge.svelte'
  import Button from '@components/ui/Button.svelte'
  import Card from '@components/ui/Card.svelte'
  import Input from '@components/ui/Input.svelte'
  import Modal from '@components/ui/Modal.svelte'
  import Select from '@components/ui/Select.svelte'
  import { getAdminKey } from '@lib/stores/auth.svelte'
  import { listTenants, createTenant, updateTenant, regenerateKey } from '@lib/api/skyscraper'
  import { formatDate, planBadgeColor } from '@lib/utils/format'
  import type { Tenant, TenantPlan } from '@lib/types'

  let tenants = $state<Tenant[]>([])
  let loading = $state(true)
  let showCreate = $state(false)
  let editTenant = $state<Tenant | null>(null)

  let newEmail = $state('')
  let newPlan = $state<string>('free')
  let newRpm = $state('60')
  let newConc = $state('5')

  const planOpts = [
    { value: 'free', label: 'Free' }, { value: 'starter', label: 'Starter' },
    { value: 'pro', label: 'Pro' }, { value: 'business', label: 'Business' },
  ]

  const planColor: Record<string, 'gray' | 'blue' | 'purple' | 'yellow'> = {
    free: 'gray', starter: 'blue', pro: 'purple', business: 'yellow',
  }

  async function load() {
    const key = getAdminKey()
    if (!key) { loading = false; return }
    loading = true
    try { tenants = await listTenants(key) } catch { /* noop */ }
    loading = false
  }

  async function handleCreate() {
    const key = getAdminKey()
    if (!key) return
    await createTenant(key, {
      email: newEmail, plan: newPlan as TenantPlan,
      rate_limit_rpm: parseInt(newRpm), concurrent_limit: parseInt(newConc),
    })
    showCreate = false
    newEmail = ''; newPlan = 'free'; newRpm = '60'; newConc = '5'
    load()
  }

  async function handleUpdate() {
    const key = getAdminKey()
    if (!key || !editTenant) return
    await updateTenant(key, editTenant.id, {
      plan: editTenant.plan, rate_limit_rpm: editTenant.rate_limit_rpm,
      concurrent_limit: editTenant.concurrent_limit, webhook_url: editTenant.webhook_url,
    })
    editTenant = null
    load()
  }

  async function handleRegen(id: string) {
    const key = getAdminKey()
    if (!key) return
    const t = await regenerateKey(key, id)
    alert(`New API key: ${t.api_key}`)
    load()
  }

  $effect(() => { load() })
</script>

<div class="space-y-4">
  <div class="flex justify-between items-center">
    <span class="text-sm text-gray-500">{tenants.length} tenants</span>
    <Button size="sm" onclick={() => (showCreate = true)}>Create Tenant</Button>
  </div>

  <Card>
    {#if loading}
      <p class="text-sm text-gray-500 animate-pulse">Loading...</p>
    {:else if tenants.length === 0}
      <p class="text-sm text-gray-500">No tenants. Configure admin key in Settings.</p>
    {:else}
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="text-left text-gray-500 text-xs uppercase tracking-wider">
              <th class="pb-3">Email</th><th class="pb-3">Plan</th><th class="pb-3">RPM</th>
              <th class="pb-3">Concurrent</th><th class="pb-3">Active</th>
              <th class="pb-3">Created</th><th class="pb-3">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-800">
            {#each tenants as t}
              <tr class="hover:bg-gray-800/40">
                <td class="py-2.5 text-gray-200">{t.email}</td>
                <td class="py-2.5"><Badge text={t.plan} color={planColor[t.plan] ?? 'gray'} /></td>
                <td class="py-2.5 text-gray-400">{t.rate_limit_rpm}</td>
                <td class="py-2.5 text-gray-400">{t.concurrent_limit}</td>
                <td class="py-2.5"><Badge text={t.active ? 'active' : 'inactive'}
                  color={t.active ? 'green' : 'red'} /></td>
                <td class="py-2.5 text-gray-500">{formatDate(t.created_at)}</td>
                <td class="py-2.5 space-x-1">
                  <Button size="sm" variant="ghost" onclick={() => (editTenant = { ...t })}>Edit</Button>
                  <Button size="sm" variant="ghost" onclick={() => handleRegen(t.id)}>Regen Key</Button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </Card>
</div>

<Modal bind:open={showCreate} title="Create Tenant">
  <div class="space-y-3">
    <Input label="Email" bind:value={newEmail} placeholder="user@example.com" />
    <Select label="Plan" options={planOpts} bind:value={newPlan} />
    <div class="grid grid-cols-2 gap-3">
      <Input label="Rate Limit (RPM)" type="number" bind:value={newRpm} />
      <Input label="Concurrent Limit" type="number" bind:value={newConc} />
    </div>
    <div class="flex justify-end gap-2 pt-2">
      <Button variant="ghost" onclick={() => (showCreate = false)}>Cancel</Button>
      <Button onclick={handleCreate}>Create</Button>
    </div>
  </div>
</Modal>

<Modal bind:open={() => editTenant !== null, (v) => { if (!v) editTenant = null }} title="Edit Tenant">
  {#if editTenant}
    <div class="space-y-3">
      <Select label="Plan" options={planOpts} bind:value={editTenant.plan} />
      <Input label="Rate Limit (RPM)" type="number" value={String(editTenant.rate_limit_rpm)}
        oninput={(e: Event) => { if (editTenant) editTenant.rate_limit_rpm = parseInt((e.target as HTMLInputElement).value) }} />
      <Input label="Concurrent Limit" type="number" value={String(editTenant.concurrent_limit)}
        oninput={(e: Event) => { if (editTenant) editTenant.concurrent_limit = parseInt((e.target as HTMLInputElement).value) }} />
      <Input label="Webhook URL" value={editTenant.webhook_url ?? ''}
        oninput={(e: Event) => { if (editTenant) editTenant.webhook_url = (e.target as HTMLInputElement).value || null }} />
      <div class="flex justify-end gap-2 pt-2">
        <Button variant="ghost" onclick={() => (editTenant = null)}>Cancel</Button>
        <Button onclick={handleUpdate}>Save</Button>
      </div>
    </div>
  {/if}
</Modal>
