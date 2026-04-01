<script lang="ts">
  let {
    value = $bindable([]),
    placeholder = 'Type and press Enter',
  }: {
    value: string[];
    placeholder?: string;
  } = $props();

  let input = $state('');

  function addTag(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      const tag = input.trim();
      if (tag && !value.includes(tag)) {
        value = [...value, tag];
      }
      input = '';
    }
  }

  function removeTag(index: number) {
    value = value.filter((_, i) => i !== index);
  }
</script>

<div style="display: flex; flex-wrap: wrap; gap: 0.5rem; padding: 0.5rem; background: var(--nb-bg3); border: 1px solid var(--nb-border); border-radius: 0.25rem; min-height: 2.5rem;">
  {#each value as tag, i}
    <span style="display: inline-flex; align-items: center; gap: 0.25rem; background: var(--nb-bg4); border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.125rem 0.5rem; font-size: 0.75rem; color: var(--nb-text2);">
      {tag}
      <button
        style="background: transparent; border: none; color: var(--nb-text3); font-size: 0.75rem; cursor: pointer; padding: 0.125rem 0.25rem; line-height: 1;"
        onmouseenter={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'var(--nb-red-text)'; }}
        onmouseleave={(e) => { (e.currentTarget as HTMLButtonElement).style.color = 'var(--nb-text3)'; }}
        onclick={() => removeTag(i)}
      >&times;</button>
    </span>
  {/each}
  <input
    type="text"
    class="nb-input"
    style="flex: 1; min-width: 120px; padding: 0.125rem 0.25rem; font-size: 0.8125rem; background: transparent; border-color: transparent;"
    {placeholder}
    bind:value={input}
    onkeydown={addTag}
  />
</div>
