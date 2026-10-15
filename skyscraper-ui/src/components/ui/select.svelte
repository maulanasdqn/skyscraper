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
    <!-- svelte-ignore a11y_label_has_associated_control -->
    <label class="text-xs font-medium text-text-muted">{label}</label>
  {/if}
  <select
    bind:value
    class="w-full rounded-xl border border-border-subtle bg-surface-raised px-3.5 py-2 text-sm
      text-text-primary transition-colors
      focus:outline-none focus:ring-1 focus:ring-accent/40 focus:border-accent/40
      appearance-none cursor-pointer"
  >
    {#each options as opt}
      <option value={opt.value}>{opt.label}</option>
    {/each}
  </select>
</div>
