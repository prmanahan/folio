<script lang="ts">
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';

  let {
    value = $bindable(''),
    label = 'Content',
  }: {
    value: string;
    label?: string;
  } = $props();

  const rendered = $derived(DOMPurify.sanitize(marked.parse(value) as string));
</script>

<label class="label text-sm font-medium" for="markdown-editor">{label}</label>
<div class="grid grid-cols-2 gap-4 min-h-[300px]">
  <textarea
    id="markdown-editor"
    class="textarea textarea-bordered w-full h-full font-mono text-sm"
    bind:value
    placeholder="Write markdown..."
  ></textarea>
  <div class="prose prose-sm bg-base-200 p-4 rounded-lg overflow-y-auto max-w-none">
    {@html rendered}
  </div>
</div>
