<script lang="ts">
  let {
    value = $bindable([]),
    placeholder = 'Enter item',
  }: {
    value: string[];
    placeholder?: string;
  } = $props();

  function addItem() {
    value = [...value, ''];
  }

  function removeItem(index: number) {
    value = value.filter((_, i) => i !== index);
  }

  function updateItem(index: number, text: string) {
    value = value.map((v, i) => (i === index ? text : v));
  }
</script>

<div class="space-y-2">
  {#each value as item, i}
    <div class="flex gap-2">
      <input
        type="text"
        class="input input-bordered input-sm flex-1"
        {placeholder}
        value={item}
        oninput={(e) => updateItem(i, e.currentTarget.value)}
      />
      <button class="btn btn-ghost btn-sm text-error" onclick={() => removeItem(i)}>&times;</button>
    </div>
  {/each}
  <button class="btn btn-ghost btn-sm text-primary" onclick={addItem}>+ Add item</button>
</div>
