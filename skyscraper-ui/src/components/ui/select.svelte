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
  <div class="relative">
    <select
      bind:value
      class="w-full rounded-lg border border-border-default bg-surface-raised/50 px-3 py-2 text-sm
        text-text-primary transition-all appearance-none cursor-pointer pr-8
        focus:outline-none focus:ring-2 focus:ring-accent/20 focus:border-accent/30"
    >
      {#each options as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
    <svg class="absolute right-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-muted pointer-events-none" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
      <path d="M19 9l-7 7-7-7" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  </div>
</div>
