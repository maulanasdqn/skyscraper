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
    <label class="text-sm font-medium text-gray-300">{label}</label>
  {/if}
  <input
    {type}
    {placeholder}
    {disabled}
    bind:value
    class="w-full rounded-lg border bg-gray-800 px-3 py-2 text-sm text-gray-100 placeholder-gray-500
      focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent
      disabled:opacity-50 disabled:cursor-not-allowed
      {error ? 'border-red-500' : 'border-gray-700'}"
  />
  {#if error}
    <p class="text-xs text-red-400">{error}</p>
  {/if}
</div>
