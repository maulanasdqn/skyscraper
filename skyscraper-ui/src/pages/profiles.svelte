<script lang="ts">
  import Badge from '@components/ui/badge.svelte'
  import Button from '@components/ui/button.svelte'
  import Input from '@components/ui/input.svelte'
  import Modal from '@components/ui/modal.svelte'
  import EmptyState from '@components/ui/empty-state.svelte'
  import { getCosmiumUrl, getCosmiumApiKey } from '@lib/stores/auth.svelte'
  import { showToast } from '@lib/stores/toast.svelte'
  import { highlightJson } from '@lib/utils/highlight'
  import {
    getCosmiumHealth, listProfiles, getProfile, deleteProfile,
    generateProfile, runFingerprintTest, runStealthTest,
    type CosmiumHealth,
  } from '@lib/api/cosmium'

  let health = $state<CosmiumHealth | null>(null)
  let profiles = $state<string[]>([])
  let loading = $state(true)
  let error = $state('')
  let selected = $state<{ name: string; data: Record<string, unknown> } | null>(null)
  let showGenerate = $state(false)
  let genName = $state('')
  let genPersona = $state('')
  let genLoading = $state(false)
  let testResult = $state<Record<string, unknown> | null>(null)

  async function load() {
    loading = true; error = ''
    const [h, p] = await Promise.allSettled([
      getCosmiumHealth(getCosmiumUrl(), getCosmiumApiKey()),
      listProfiles(getCosmiumUrl(), getCosmiumApiKey()),
    ])
    if (h.status === 'fulfilled') health = h.value
    if (p.status === 'fulfilled') profiles = p.value.profiles
    else error = 'Cannot reach Cosmium'
    loading = false
  }

  async function viewProfile(name: string) {
    try {
      const data = await getProfile(getCosmiumUrl(), name, getCosmiumApiKey())
      selected = { name, data }
    } catch (e) { error = e instanceof Error ? e.message : 'Failed' }
  }

  async function handleDelete(name: string) {
    if (!confirm(`Delete "${name}"?`)) return
    await deleteProfile(getCosmiumUrl(), name, getCosmiumApiKey())
    selected = null; load()
    showToast('success', `Profile "${name}" deleted`)
  }

  async function handleGenerate() {
    genLoading = true
    try {
      await generateProfile(getCosmiumUrl(), genPersona, genName, getCosmiumApiKey())
      showGenerate = false; genName = ''; genPersona = ''; load()
      showToast('success', 'Profile generated')
    } catch (e) { showToast('error', e instanceof Error ? e.message : 'Generation failed') }
    genLoading = false
  }

  async function runTest(type: 'fingerprint' | 'stealth') {
    try {
      testResult = type === 'fingerprint'
        ? await runFingerprintTest(getCosmiumUrl(), getCosmiumApiKey())
        : await runStealthTest(getCosmiumUrl(), getCosmiumApiKey())
    } catch (e) { testResult = { error: e instanceof Error ? e.message : 'Test failed' } }
  }

  $effect(() => { load() })
</script>

<div class="space-y-4 animate-fade-in">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-2">
      {#if health}
        <Badge text={health.status === 'ok' ? 'connected' : 'error'} color={health.status === 'ok' ? 'green' : 'red'} />
        {#if health.llm_configured}<Badge text="AI" color="purple" />{/if}
      {/if}
    </div>
    <div class="flex gap-2">
      <Button size="sm" onclick={() => (showGenerate = true)}>Generate</Button>
      <Button size="sm" variant="secondary" onclick={() => runTest('fingerprint')}>Fingerprint</Button>
      <Button size="sm" variant="secondary" onclick={() => runTest('stealth')}>Stealth</Button>
    </div>
  </div>

  {#if error}<p class="text-sm text-danger">{error}</p>{/if}

  <div class="grid grid-cols-1 lg:grid-cols-5 gap-3">
    <div class="lg:col-span-2 rounded-xl border border-border-subtle bg-surface overflow-hidden">
      <div class="px-5 py-3 border-b border-border-subtle">
        <span class="text-xs text-text-muted">{profiles.length} profiles</span>
      </div>
      {#if loading}
        <div class="p-6 text-center">
          <svg class="animate-spin h-5 w-5 mx-auto text-accent" viewBox="0 0 24 24" fill="none">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
        </div>
      {:else if profiles.length === 0}
        <EmptyState icon="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
          title="No profiles" description="Generate one to get started" />
      {:else}
        <div class="max-h-80 overflow-y-auto">
          {#each profiles as name}
            <button
              class="w-full text-left px-5 py-2.5 text-sm border-b border-border-subtle/50 transition-colors
                {selected?.name === name ? 'bg-accent-muted text-accent' : 'text-text-secondary hover:bg-surface-raised/50'}"
              onclick={() => viewProfile(name)}>
              {name}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="lg:col-span-3 rounded-xl border border-border-subtle bg-surface overflow-hidden">
      {#if selected}
        <div class="flex items-center justify-between px-5 py-3 border-b border-border-subtle">
          <span class="text-sm text-text-primary font-medium">{selected.name}</span>
          <button class="text-xs text-danger hover:text-danger/80 transition-colors" onclick={() => handleDelete(selected!.name)}>Delete</button>
        </div>
        <pre class="p-5 text-xs text-text-secondary font-mono overflow-auto max-h-80 leading-relaxed">{@html highlightJson(selected.data)}</pre>
      {:else}
        <EmptyState icon="M15 12a3 3 0 11-6 0 3 3 0 016 0z" title="Select a profile" description="Choose a profile from the list to view its configuration" />
      {/if}
    </div>
  </div>

  {#if testResult}
    <div class="rounded-xl border border-border-subtle bg-surface overflow-hidden animate-fade-in">
      <div class="flex items-center justify-between px-5 py-3 border-b border-border-subtle">
        <span class="text-xs text-text-muted uppercase tracking-wider">Test Result</span>
        <button class="text-xs text-text-muted hover:text-text-secondary transition-colors" onclick={() => (testResult = null)}>Dismiss</button>
      </div>
      <pre class="p-5 text-xs text-text-secondary font-mono overflow-auto max-h-60 leading-relaxed">{@html highlightJson(testResult)}</pre>
    </div>
  {/if}
</div>

<Modal bind:open={showGenerate} title="Generate Profile">
  <div class="space-y-3">
    <Input label="Profile Name" bind:value={genName} placeholder="chrome-linux-dev" />
    <Input label="Persona" bind:value={genPersona} placeholder="Developer using Linux with Chrome" />
    <div class="flex justify-end gap-2 pt-2">
      <Button variant="ghost" onclick={() => (showGenerate = false)}>Cancel</Button>
      <Button onclick={handleGenerate} loading={genLoading}>Generate</Button>
    </div>
  </div>
</Modal>
