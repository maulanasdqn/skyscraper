<script lang="ts">
  let {
    label = '',
    options = [],
    value = $bindable(''),
  }: {
    label?: string
    options: { value: string; label: string }[]
    value?: string
  } = $props()
</script>

<div class="flex flex-col gap-1.5">
  {#if label}
    <label class="text-sm font-medium text-gray-300">{label}</label>
  {/if}
  <select
    bind:value
    class="w-full rounded-lg border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-100
      focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent
      appearance-none cursor-pointer"
  >
    {#each options as opt}
      <option value={opt.value}>{opt.label}</option>
    {/each}
  </select>
</div>
