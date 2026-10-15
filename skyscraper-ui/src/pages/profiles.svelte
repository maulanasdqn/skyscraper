<script lang="ts">
  import Badge from '@components/ui/badge.svelte'
  import Button from '@components/ui/button.svelte'
  import Input from '@components/ui/input.svelte'
  import Modal from '@components/ui/modal.svelte'
  import { getCosmiumUrl, getCosmiumApiKey } from '@lib/stores/auth.svelte'
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
  }

  async function handleGenerate() {
    genLoading = true
    try {
      await generateProfile(getCosmiumUrl(), genPersona, genName, getCosmiumApiKey())
      showGenerate = false; genName = ''; genPersona = ''; load()
    } catch (e) { error = e instanceof Error ? e.message : 'Generation failed' }
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

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-xl font-bold text-text-primary">Profiles</h1>
      {#if health}
        <div class="flex items-center gap-2 mt-1">
          <Badge text={health.status === 'ok' ? 'connected' : 'error'} color={health.status === 'ok' ? 'green' : 'red'} />
          {#if health.llm_configured}<Badge text="AI" color="purple" />{/if}
        </div>
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
      <div class="px-4 py-2.5 border-b border-border-subtle">
        <span class="text-xs text-text-muted">{profiles.length} profiles</span>
      </div>
      {#if loading}
        <div class="p-4 text-sm text-text-muted">Loading...</div>
      {:else if profiles.length === 0}
        <div class="p-4 text-sm text-text-muted">No profiles</div>
      {:else}
        <div class="max-h-80 overflow-y-auto">
          {#each profiles as name}
            <button
              class="w-full text-left px-4 py-2 text-sm border-b border-border-subtle/50 transition-colors
                {selected?.name === name ? 'bg-accent-muted text-accent' : 'text-text-secondary hover:bg-surface-raised'}"
              onclick={() => viewProfile(name)}>
              {name}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <div class="lg:col-span-3 rounded-xl border border-border-subtle bg-surface overflow-hidden">
      {#if selected}
        <div class="flex items-center justify-between px-4 py-2.5 border-b border-border-subtle">
          <span class="text-sm text-text-secondary font-medium">{selected.name}</span>
          <button class="text-xs text-danger hover:underline" onclick={() => handleDelete(selected!.name)}>Delete</button>
        </div>
        <pre class="p-4 text-xs text-text-secondary font-mono overflow-auto max-h-80 leading-relaxed">{JSON.stringify(selected.data, null, 2)}</pre>
      {:else}
        <div class="p-8 text-center text-sm text-text-muted">Select a profile</div>
      {/if}
    </div>
  </div>

  {#if testResult}
    <div class="rounded-xl border border-border-subtle bg-surface overflow-hidden">
      <div class="flex items-center justify-between px-4 py-2.5 border-b border-border-subtle">
        <span class="text-xs text-text-muted uppercase tracking-wider">Test Result</span>
        <button class="text-xs text-text-muted hover:text-text-secondary" onclick={() => (testResult = null)}>Dismiss</button>
      </div>
      <pre class="p-4 text-xs text-text-secondary font-mono overflow-auto max-h-60 leading-relaxed">{JSON.stringify(testResult, null, 2)}</pre>
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
