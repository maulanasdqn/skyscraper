<script lang="ts">
  let {
    label = '',
    type = 'text',
    placeholder = '',
    value = $bindable(''),
    error = '',
    disabled = false,
  }: {
    label?: string
    type?: string
    placeholder?: string
    value?: string
    error?: string
    disabled?: boolean
  } = $props()
</script>

<div class="flex flex-col gap-1.5">
  {#if label}
    <!-- svelte-ignore a11y_label_has_associated_control -->
    <label class="text-xs font-medium text-text-muted">{label}</label>
  {/if}
  <input
    {type}
    {placeholder}
    {disabled}
    bind:value
    class="w-full rounded-lg border bg-surface-raised/50 px-3 py-2 text-sm text-text-primary
      placeholder-text-muted/60 transition-all
      focus:outline-none focus:ring-2 focus:ring-accent/20 focus:border-accent/30
      disabled:opacity-40 disabled:cursor-not-allowed
      {error ? 'border-danger/40' : 'border-border-default'}"
  />
  {#if error}
    <p class="text-xs text-danger">{error}</p>
  {/if}
</div>
