<script lang="ts">
  import Button from '@components/ui/Button.svelte'
  import Card from '@components/ui/Card.svelte'
  import Input from '@components/ui/Input.svelte'
  import Badge from '@components/ui/Badge.svelte'
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
    setAdminKey(adminVal)
    setTenantApiKey(tenantVal)
    setCosmiumUrl(cosmiumVal)
    setParallaxUrl(parallaxVal)
    setCosmiumApiKey(cosmiumKeyVal)
    saved = true
    setTimeout(() => (saved = false), 2000)
  }

  function handleClear() {
    adminVal = ''; tenantVal = ''; cosmiumVal = 'http://localhost:3000'
    parallaxVal = 'http://localhost:3001'; cosmiumKeyVal = ''
    handleSave()
  }

  async function testSkyScraper() {
    skyStatus = 'testing'
    try { await getHealth(); skyStatus = 'ok' }
    catch { skyStatus = 'fail' }
  }

  async function testCosmium() {
    cosStatus = 'testing'
    try { await getCosmiumHealth(cosmiumVal, cosmiumKeyVal); cosStatus = 'ok' }
    catch { cosStatus = 'fail' }
  }

  async function testParallax() {
    plxStatus = 'testing'
    try { await getParallaxCarriers(parallaxVal); plxStatus = 'ok' }
    catch { plxStatus = 'fail' }
  }

  function statusBadge(s: string): { text: string; color: 'green' | 'red' | 'yellow' | 'gray' } {
    if (s === 'ok') return { text: 'Connected', color: 'green' }
    if (s === 'fail') return { text: 'Failed', color: 'red' }
    if (s === 'testing') return { text: 'Testing...', color: 'yellow' }
    return { text: 'Not tested', color: 'gray' }
  }
</script>

<div class="max-w-2xl space-y-6">
  <Card title="API Keys">
    <div class="space-y-4">
      <Input label="Admin API Key" type="password" bind:value={adminVal}
        placeholder="sk_admin_..." />
      <Input label="Tenant API Key" type="password" bind:value={tenantVal}
        placeholder="sk_..." />
      <p class="text-xs text-gray-500">
        Admin key manages tenants and system. Tenant key submits scrape jobs.
      </p>
    </div>
  </Card>

  <Card title="Service Connections">
    <div class="space-y-4">
      <div class="flex items-end gap-3">
        <div class="flex-1">
          <p class="text-xs text-gray-500 mb-2">SkyScraper API (this server)</p>
        </div>
        <Button size="sm" variant="secondary" onclick={testSkyScraper}
          loading={skyStatus === 'testing'}>Test</Button>
        <Badge {...statusBadge(skyStatus)} />
      </div>

      <div class="border-t border-gray-800 pt-4">
        <div class="flex items-end gap-3">
          <div class="flex-1"><Input label="Cosmium URL" bind:value={cosmiumVal} placeholder="http://localhost:3000" /></div>
          <Button size="sm" variant="secondary" onclick={testCosmium}
            loading={cosStatus === 'testing'}>Test</Button>
          <Badge {...statusBadge(cosStatus)} />
        </div>
        <div class="mt-3">
          <Input label="Cosmium API Key" type="password" bind:value={cosmiumKeyVal}
            placeholder="Optional" />
        </div>
      </div>

      <div class="border-t border-gray-800 pt-4">
        <div class="flex items-end gap-3">
          <div class="flex-1"><Input label="Parallax URL" bind:value={parallaxVal} placeholder="http://localhost:3001" /></div>
          <Button size="sm" variant="secondary" onclick={testParallax}
            loading={plxStatus === 'testing'}>Test</Button>
          <Badge {...statusBadge(plxStatus)} />
        </div>
      </div>
    </div>
  </Card>

  <div class="flex items-center gap-3">
    <Button onclick={handleSave}>Save Settings</Button>
    <Button variant="danger" onclick={handleClear}>Reset to Defaults</Button>
    {#if saved}
      <span class="text-sm text-green-400">Saved</span>
    {/if}
  </div>
</div>
