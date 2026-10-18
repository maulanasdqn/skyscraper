<script lang="ts">
  import Button from '@components/ui/button.svelte'
  import Input from '@components/ui/input.svelte'
  import Badge from '@components/ui/badge.svelte'
  import { showToast } from '@lib/stores/toast.svelte'
  import {
    getAdminKey, setAdminKey,
    getTenantApiKey, setTenantApiKey,
    getCosmiumUrl, setCosmiumUrl,
    getParallaxUrl, setParallaxUrl,
    getCosmiumApiKey, setCosmiumApiKey,
    getWebhookUrl, setWebhookUrl,
  } from '@lib/stores/auth.svelte'
  import { getHealth } from '@lib/api/skyscraper'
  import { getCosmiumHealth } from '@lib/api/cosmium'
  import { getParallaxCarriers } from '@lib/api/parallax'

  let adminVal = $state(getAdminKey())
  let tenantVal = $state(getTenantApiKey())
  let cosmiumVal = $state(getCosmiumUrl())
  let parallaxVal = $state(getParallaxUrl())
  let cosmiumKeyVal = $state(getCosmiumApiKey())
  let webhookVal = $state(getWebhookUrl())
  let webhookValid = $state<'idle' | 'valid' | 'invalid'>('idle')

  let skyStatus = $state<'idle' | 'testing' | 'ok' | 'fail'>('idle')
  let cosStatus = $state<'idle' | 'testing' | 'ok' | 'fail'>('idle')
  let plxStatus = $state<'idle' | 'testing' | 'ok' | 'fail'>('idle')

  function handleSave() {
    setAdminKey(adminVal); setTenantApiKey(tenantVal)
    setCosmiumUrl(cosmiumVal); setParallaxUrl(parallaxVal); setCosmiumApiKey(cosmiumKeyVal)
    setWebhookUrl(webhookVal)
    showToast('success', 'Settings saved')
  }

  function testWebhook() {
    try { new URL(webhookVal); webhookValid = 'valid' } catch { webhookValid = 'invalid' }
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

<div class="space-y-6 max-w-xl animate-fade-in">
  <div class="rounded-xl border border-border-subtle bg-surface divide-y divide-border-subtle">
    <div class="p-5 space-y-3">
      <h3 class="text-xs text-text-muted uppercase tracking-wider font-medium">API Keys</h3>
      <Input label="Admin Key" type="password" bind:value={adminVal} placeholder="sk_admin_..." />
      <Input label="Tenant Key" type="password" bind:value={tenantVal} placeholder="sk_..." />
    </div>

    <div class="p-5 space-y-4">
      <h3 class="text-xs text-text-muted uppercase tracking-wider font-medium">Services</h3>

      <div class="flex items-end gap-3">
        <div class="flex-1"><span class="text-sm text-text-secondary">SkyScraper API</span></div>
        <button class="text-xs text-text-muted hover:text-text-secondary transition-colors" onclick={testSky}>Test</button>
        <Badge {...statusBadge(skyStatus)} />
      </div>

      <div class="space-y-2">
        <div class="flex items-end gap-3">
          <div class="flex-1"><Input label="Cosmium" bind:value={cosmiumVal} placeholder="http://localhost:3000" /></div>
          <button class="text-xs text-text-muted hover:text-text-secondary mb-1 transition-colors" onclick={testCos}>Test</button>
          <div class="mb-1"><Badge {...statusBadge(cosStatus)} /></div>
        </div>
        <Input label="Cosmium API Key" type="password" bind:value={cosmiumKeyVal} placeholder="Optional" />
      </div>

      <div class="flex items-end gap-3">
        <div class="flex-1"><Input label="Parallax" bind:value={parallaxVal} placeholder="http://localhost:3001" /></div>
        <button class="text-xs text-text-muted hover:text-text-secondary mb-1 transition-colors" onclick={testPlx}>Test</button>
        <div class="mb-1"><Badge {...statusBadge(plxStatus)} /></div>
      </div>
    </div>

    <div class="p-5 space-y-3">
      <h3 class="text-xs text-text-muted uppercase tracking-wider font-medium">Webhooks</h3>
      <p class="text-xs text-text-muted">Receive POST notifications when jobs complete or fail</p>
      <div class="flex items-end gap-3">
        <div class="flex-1"><Input label="Webhook URL" bind:value={webhookVal} placeholder="https://your-server.com/webhook" /></div>
        <button class="text-xs text-text-muted hover:text-text-secondary mb-1 transition-colors" onclick={testWebhook}>Validate</button>
        {#if webhookValid === 'valid'}
          <div class="mb-1"><Badge text="Valid" color="green" /></div>
        {:else if webhookValid === 'invalid'}
          <div class="mb-1"><Badge text="Invalid" color="red" /></div>
        {/if}
      </div>
      <p class="text-[10px] text-text-muted">Payload: {"{"} job_id, status, result, error {"}"}</p>
    </div>
  </div>

  <Button onclick={handleSave}>Save Settings</Button>
</div>
