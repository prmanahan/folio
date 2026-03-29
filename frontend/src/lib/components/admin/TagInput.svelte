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

<div class="flex flex-wrap gap-2 p-2 bg-base-300 rounded-lg min-h-[2.5rem]">
  {#each value as tag, i}
    <span class="badge badge-lg gap-1">
      {tag}
      <button class="btn btn-ghost btn-xs" onclick={() => removeTag(i)}>&times;</button>
    </span>
  {/each}
  <input
    type="text"
    class="input input-sm input-ghost flex-1 min-w-[120px]"
    {placeholder}
    bind:value={input}
    onkeydown={addTag}
  />
</div>
