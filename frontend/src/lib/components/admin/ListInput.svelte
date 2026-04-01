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
        class="nb-input"
        style="padding: 0.375rem 0.625rem; font-size: 0.8125rem; flex: 1;"
        {placeholder}
        value={item}
        oninput={(e) => updateItem(i, e.currentTarget.value)}
      />
      <button
        style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.25rem 0.5rem; color: var(--nb-red-text); font-size: 0.75rem; cursor: pointer;"
        onmouseenter={(e) => { (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--nb-red)'; }}
        onmouseleave={(e) => { (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--nb-border)'; }}
        onclick={() => removeItem(i)}
      >&times;</button>
    </div>
  {/each}
  <button
    style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.25rem 0.5rem; color: var(--nb-text2); font-size: 0.75rem; cursor: pointer;"
    onmouseenter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'var(--nb-text)'; }}
    onmouseleave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'var(--nb-text2)'; }}
    onclick={addItem}
  >+ Add item</button>
</div>
