<script lang="ts">
  import Badge from '@components/ui/badge.svelte'
  import Button from '@components/ui/button.svelte'
  import Input from '@components/ui/input.svelte'
  import Modal from '@components/ui/modal.svelte'
  import Select from '@components/ui/select.svelte'
  import EmptyState from '@components/ui/empty-state.svelte'
  import { showToast } from '@lib/stores/toast.svelte'
  import { getAdminKey } from '@lib/stores/auth.svelte'
  import { listTenants, createTenant, updateTenant, regenerateKey } from '@lib/api/skyscraper'
  import { formatDate } from '@lib/utils/format'
  import type { Tenant, TenantPlan } from '@lib/types'

  let tenants = $state<Tenant[]>([])
  let loading = $state(true)
  let showCreate = $state(false)
  let editTenant = $state<Tenant | null>(null)

  let newEmail = $state('')
  let newPlan = $state<string>('free')
  let newRpm = $state('60')
  let newConc = $state('5')
  let editRpm = $state('')
  let editConc = $state('')

  const planOpts = [
    { value: 'free', label: 'Free' }, { value: 'starter', label: 'Starter' },
    { value: 'pro', label: 'Pro' }, { value: 'business', label: 'Business' },
  ]
  const planColor: Record<string, 'gray' | 'blue' | 'purple' | 'orange'> = {
    free: 'gray', starter: 'blue', pro: 'purple', business: 'orange',
  }

  async function load() {
    const key = getAdminKey()
    if (!key) { loading = false; return }
    loading = true
    try { tenants = await listTenants(key) } catch {}
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
    showToast('success', 'Tenant created')
    load()
  }

  async function handleUpdate() {
    const key = getAdminKey()
    if (!key || !editTenant) return
    await updateTenant(key, editTenant.id, {
      plan: editTenant.plan, rate_limit_rpm: parseInt(editRpm) || editTenant.rate_limit_rpm,
      concurrent_limit: parseInt(editConc) || editTenant.concurrent_limit, webhook_url: editTenant.webhook_url,
    })
    editTenant = null
    showToast('success', 'Tenant updated')
    load()
  }

  async function handleRegen(id: string) {
    const key = getAdminKey()
    if (!key) return
    const t = await regenerateKey(key, id)
    showToast('success', `New API key: ${t.api_key}`)
    load()
  }

  $effect(() => { load() })
</script>

<div class="space-y-4 animate-fade-in">
  <div class="flex items-center justify-end">
    <Button size="sm" onclick={() => (showCreate = true)}>Create Tenant</Button>
  </div>

  <div class="rounded-xl border border-border-subtle bg-surface overflow-hidden">
    {#if loading}
      <div class="p-8 text-center">
        <svg class="animate-spin h-5 w-5 mx-auto text-accent" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
      </div>
    {:else if tenants.length === 0}
      <EmptyState
        icon="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197"
        title="No tenants yet"
        description="Create your first tenant to start issuing API keys"
      >
        <Button size="sm" onclick={() => (showCreate = true)}>Create Tenant</Button>
      </EmptyState>
    {:else}
      <table class="w-full">
        <thead>
          <tr class="border-b border-border-subtle">
            {#each ['Email', 'Plan', 'RPM', 'Active', 'Actions'] as h}
              <th class="text-left text-[11px] text-text-muted uppercase tracking-wider px-4 py-2.5">{h}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each tenants as t}
            <tr class="border-b border-border-subtle/50 hover:bg-surface-raised/50 transition-colors">
              <td class="px-4 py-2.5 text-sm text-text-secondary">{t.email}</td>
              <td class="px-4 py-2.5"><Badge text={t.plan} color={planColor[t.plan] ?? 'gray'} /></td>
              <td class="px-4 py-2.5 text-sm text-text-muted tabular-nums">{t.rate_limit_rpm}</td>
              <td class="px-4 py-2.5"><Badge text={t.active ? 'active' : 'inactive'}
                color={t.active ? 'green' : 'red'} /></td>
              <td class="px-4 py-2.5 space-x-2">
                <button class="text-xs text-text-muted hover:text-text-secondary transition-colors" onclick={() => { editTenant = { ...t }; editRpm = String(t.rate_limit_rpm); editConc = String(t.concurrent_limit) }}>Edit</button>
                <button class="text-xs text-text-muted hover:text-text-secondary transition-colors" onclick={() => handleRegen(t.id)}>Regen</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<Modal bind:open={showCreate} title="Create Tenant">
  <div class="space-y-3">
    <Input label="Email" bind:value={newEmail} placeholder="user@example.com" />
    <Select label="Plan" options={planOpts} bind:value={newPlan} />
    <div class="grid grid-cols-2 gap-3">
      <Input label="RPM Limit" type="number" bind:value={newRpm} />
      <Input label="Concurrent" type="number" bind:value={newConc} />
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
      <Input label="RPM Limit" type="number" bind:value={editRpm} />
      <Input label="Concurrent" type="number" bind:value={editConc} />
      <div class="flex justify-end gap-2 pt-2">
        <Button variant="ghost" onclick={() => (editTenant = null)}>Cancel</Button>
        <Button onclick={handleUpdate}>Save</Button>
      </div>
    </div>
  {/if}
</Modal>
