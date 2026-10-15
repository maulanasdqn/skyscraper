<script lang="ts">
  import Badge from '../components/ui/Badge.svelte'
  import Button from '../components/ui/Button.svelte'
  import Card from '../components/ui/Card.svelte'
  import Input from '../components/ui/Input.svelte'
  import Modal from '../components/ui/Modal.svelte'
  import { getCosmiumUrl, getCosmiumApiKey } from '../lib/stores/auth.svelte'
  import {
    getCosmiumHealth, listProfiles, getProfile, deleteProfile,
    generateProfile, runFingerprintTest, runStealthTest,
    type CosmiumHealth,
  } from '../lib/api/cosmium'

  let health = $state<CosmiumHealth | null>(null)
  let profiles = $state<string[]>([])
  let loading = $state(true)
  let error = $state('')
  let selectedProfile = $state<{ name: string; data: Record<string, unknown> } | null>(null)
  let showGenerate = $state(false)
  let genName = $state('')
  let genPersona = $state('')
  let genLoading = $state(false)
  let testResult = $state<Record<string, unknown> | null>(null)

  async function load() {
    loading = true
    error = ''
    const url = getCosmiumUrl()
    const key = getCosmiumApiKey()
    try {
      const [h, p] = await Promise.allSettled([
        getCosmiumHealth(url, key),
        listProfiles(url, key),
      ])
      if (h.status === 'fulfilled') health = h.value
      if (p.status === 'fulfilled') profiles = p.value.profiles
    } catch (e) { error = e instanceof Error ? e.message : 'Cannot reach Cosmium' }
    loading = false
  }

  async function viewProfile(name: string) {
    try {
      const data = await getProfile(getCosmiumUrl(), name, getCosmiumApiKey())
      selectedProfile = { name, data }
    } catch (e) { error = e instanceof Error ? e.message : 'Failed to load profile' }
  }

  async function handleDelete(name: string) {
    if (!confirm(`Delete profile "${name}"?`)) return
    try {
      await deleteProfile(getCosmiumUrl(), name, getCosmiumApiKey())
      selectedProfile = null
      load()
    } catch (e) { error = e instanceof Error ? e.message : 'Failed' }
  }

  async function handleGenerate() {
    genLoading = true
    try {
      await generateProfile(getCosmiumUrl(), genPersona, genName, getCosmiumApiKey())
      showGenerate = false
      genName = ''; genPersona = ''
      load()
    } catch (e) { error = e instanceof Error ? e.message : 'Generation failed' }
    genLoading = false
  }

  async function runTest(type: 'fingerprint' | 'stealth') {
    const url = getCosmiumUrl()
    const key = getCosmiumApiKey()
    try {
      testResult = type === 'fingerprint'
        ? await runFingerprintTest(url, key)
        : await runStealthTest(url, key)
    } catch (e) { testResult = { error: e instanceof Error ? e.message : 'Test failed' } }
  }

  $effect(() => { load() })
</script>

<div class="space-y-4">
  {#if health}
    <div class="flex items-center gap-3 text-sm">
      <Badge text={health.status} color={health.status === 'ok' ? 'green' : 'red'} />
      <span class="text-gray-400">Cosmium {health.version}</span>
      <Badge text={health.binary_found ? 'binary OK' : 'no binary'} color={health.binary_found ? 'green' : 'red'} />
      {#if health.llm_configured}
        <Badge text="AI enabled" color="purple" />
      {/if}
    </div>
  {/if}

  {#if error}<p class="text-sm text-red-400">{error}</p>{/if}

  <div class="flex gap-2">
    <Button size="sm" onclick={() => (showGenerate = true)}>Generate Profile</Button>
    <Button size="sm" variant="secondary" onclick={() => runTest('fingerprint')}>Fingerprint Test</Button>
    <Button size="sm" variant="secondary" onclick={() => runTest('stealth')}>Stealth Test</Button>
    <Button size="sm" variant="ghost" onclick={load}>Refresh</Button>
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <Card title="Profiles ({profiles.length})">
      {#if loading}
        <p class="text-sm text-gray-500 animate-pulse">Loading...</p>
      {:else if profiles.length === 0}
        <p class="text-sm text-gray-500">No profiles found. Configure Cosmium URL in Settings.</p>
      {:else}
        <div class="space-y-1 max-h-96 overflow-y-auto">
          {#each profiles as name}
            <button
              class="w-full text-left px-3 py-2 rounded-lg text-sm transition-colors
                {selectedProfile?.name === name
                  ? 'bg-indigo-600/20 text-indigo-400'
                  : 'text-gray-300 hover:bg-gray-800'}"
              onclick={() => viewProfile(name)}>
              {name}
            </button>
          {/each}
        </div>
      {/if}
    </Card>

    <Card title={selectedProfile ? selectedProfile.name : 'Profile Details'}>
      {#if selectedProfile}
        <div class="flex gap-2 mb-3">
          <Button size="sm" variant="danger" onclick={() => handleDelete(selectedProfile!.name)}>Delete</Button>
        </div>
        <pre class="bg-gray-800 rounded-lg p-3 text-xs text-gray-300 overflow-auto max-h-80 whitespace-pre-wrap"
          >{JSON.stringify(selectedProfile.data, null, 2)}</pre>
      {:else}
        <p class="text-sm text-gray-500">Select a profile to view details</p>
      {/if}
    </Card>
  </div>

  {#if testResult}
    <Card title="Test Result">
      <pre class="bg-gray-800 rounded-lg p-3 text-xs text-green-400 overflow-auto max-h-60 whitespace-pre-wrap"
        >{JSON.stringify(testResult, null, 2)}</pre>
      <Button size="sm" variant="ghost" onclick={() => (testResult = null)} class="mt-2">Dismiss</Button>
    </Card>
  {/if}
</div>

<Modal bind:open={showGenerate} title="Generate Profile (AI)">
  <div class="space-y-3">
    <Input label="Profile Name" bind:value={genName} placeholder="chrome-linux-dev" />
    <Input label="Persona" bind:value={genPersona} placeholder="Developer using Linux with Chrome" />
    <div class="flex justify-end gap-2 pt-2">
      <Button variant="ghost" onclick={() => (showGenerate = false)}>Cancel</Button>
      <Button onclick={handleGenerate} loading={genLoading}>Generate</Button>
    </div>
  </div>
</Modal>
