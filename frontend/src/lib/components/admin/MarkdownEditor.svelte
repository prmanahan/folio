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
    class="nb-input"
    style="font-family: 'IBM Plex Mono', monospace; font-size: 0.8125rem; height: 100%; resize: vertical;"
    bind:value
    placeholder="Write markdown..."
  ></textarea>
  <div class="prose prose-sm bg-base-200 p-4 rounded-lg overflow-y-auto max-w-none">
    {@html rendered}
  </div>
</div>
