<script lang="ts">
  import Button from '@components/ui/button.svelte'
  import Input from '@components/ui/input.svelte'
  import Badge from '@components/ui/badge.svelte'
  import {
    getAdminKey, setAdminKey,
    getTenantApiKey, setTenantApiKey,
    getCosmiumUrl, setCosmiumUrl,
    getParallaxUrl, setParallaxUrl,
    getCosmiumApiKey, setCosmiumApiKey,
  } from '@lib/stores/auth.svelte'
  import { getHealth } from '@lib/api/skyscraper'
  import { getCosmiumHealth } from '@lib/api/cosmium'
  import { getParallaxCarriers } from '@lib/api/parallax'

  let adminVal = $state(getAdminKey())
  let tenantVal = $state(getTenantApiKey())
  let cosmiumVal = $state(getCosmiumUrl())
  let parallaxVal = $state(getParallaxUrl())
  let cosmiumKeyVal = $state(getCosmiumApiKey())

  let skyStatus = $state<'idle' | 'testing' | 'ok' | 'fail'>('idle')
  let cosStatus = $state<'idle' | 'testing' | 'ok' | 'fail'>('idle')
  let plxStatus = $state<'idle' | 'testing' | 'ok' | 'fail'>('idle')
  let saved = $state(false)

  function handleSave() {
    setAdminKey(adminVal); setTenantApiKey(tenantVal)
    setCosmiumUrl(cosmiumVal); setParallaxUrl(parallaxVal); setCosmiumApiKey(cosmiumKeyVal)
    saved = true; setTimeout(() => (saved = false), 2000)
  }

  async function testSky() { skyStatus = 'testing'; try { await getHealth(); skyStatus = 'ok' } catch { skyStatus = 'fail' } }
  async function testCos() { cosStatus = 'testing'; try { await getCosmiumHealth(cosmiumVal, cosmiumKeyVal); cosStatus = 'ok' } catch { cosStatus = 'fail' } }
  async function testPlx() { plxStatus = 'testing'; try { await getParallaxCarriers(parallaxVal); plxStatus = 'ok' } catch { plxStatus = 'fail' } }

  function statusBadge(s: string): { text: string; color: 'green' | 'red' | 'yellow' | 'gray' } {
    if (s === 'ok') return { text: 'Connected', color: 'green' }
    if (s === 'fail') return { text: 'Failed', color: 'red' }
    if (s === 'testing') return { text: 'Testing', color: 'yellow' }
    return { text: '—', color: 'gray' }
  }

  $effect(() => { testSky(); testCos(); testPlx() })
</script>

<div class="space-y-6 max-w-xl">
  <h1 class="text-xl font-bold text-text-primary">Settings</h1>

  <div class="rounded-xl border border-border-subtle bg-surface divide-y divide-border-subtle">
    <div class="p-4 space-y-3">
      <h3 class="text-xs text-text-muted uppercase tracking-wider">API Keys</h3>
      <Input label="Admin Key" type="password" bind:value={adminVal} placeholder="sk_admin_..." />
      <Input label="Tenant Key" type="password" bind:value={tenantVal} placeholder="sk_..." />
    </div>

    <div class="p-4 space-y-4">
      <h3 class="text-xs text-text-muted uppercase tracking-wider">Services</h3>

      <div class="flex items-end gap-3">
        <div class="flex-1"><span class="text-sm text-text-secondary">SkyScraper API</span></div>
        <button class="text-xs text-text-muted hover:text-text-secondary" onclick={testSky}>Test</button>
        <Badge {...statusBadge(skyStatus)} />
      </div>

      <div class="space-y-2">
        <div class="flex items-end gap-3">
          <div class="flex-1"><Input label="Cosmium" bind:value={cosmiumVal} placeholder="http://localhost:3000" /></div>
          <button class="text-xs text-text-muted hover:text-text-secondary mb-1" onclick={testCos}>Test</button>
          <div class="mb-1"><Badge {...statusBadge(cosStatus)} /></div>
        </div>
        <Input label="Cosmium API Key" type="password" bind:value={cosmiumKeyVal} placeholder="Optional" />
      </div>

      <div class="flex items-end gap-3">
        <div class="flex-1"><Input label="Parallax" bind:value={parallaxVal} placeholder="http://localhost:3001" /></div>
        <button class="text-xs text-text-muted hover:text-text-secondary mb-1" onclick={testPlx}>Test</button>
        <div class="mb-1"><Badge {...statusBadge(plxStatus)} /></div>
      </div>
    </div>
  </div>

  <div class="flex items-center gap-3">
    <Button onclick={handleSave}>Save</Button>
    {#if saved}<span class="text-xs text-success">Saved</span>{/if}
  </div>
</div>
