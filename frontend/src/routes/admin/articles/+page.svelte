<script lang="ts">
  import { onMount } from 'svelte';
  import { listArticles, createArticle, updateArticle, deleteArticle } from '$lib/admin-api';
  import type { ArticleFull, ArticleInput } from '$lib/admin-types';
  import MarkdownEditor from '$lib/components/admin/MarkdownEditor.svelte';
  import TagInput from '$lib/components/admin/TagInput.svelte';
  import Toast from '$lib/components/admin/Toast.svelte';

  let items = $state<ArticleFull[]>([]);
  let mode = $state<'list' | 'edit'>('list');
  let editingId = $state<number | null>(null);
  let form = $state<ArticleInput>(emptyForm());
  let loading = $state(true);
  let saving = $state(false);
  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');

  function emptyForm(): ArticleInput {
    return {
      title: '',
      slug: null,
      summary: '',
      content: '',
      tags: [],
      published_at: null,
      published: false,
    };
  }

  async function loadItems() {
    items = await listArticles();
  }

  onMount(async () => {
    try {
      await loadItems();
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to load articles';
      toastType = 'error';
    } finally {
      loading = false;
    }
  });

  function startCreate() {
    form = emptyForm();
    editingId = null;
    mode = 'edit';
  }

  function startEdit(item: ArticleFull) {
    editingId = item.id;
    form = {
      title: item.title,
      slug: item.slug,
      summary: item.summary,
      content: item.content,
      tags: Array.isArray(item.tags) ? item.tags : [],
      published_at: item.published_at,
      published: item.published,
    };
    mode = 'edit';
  }

  async function handleSave() {
    saving = true;
    try {
      const payload = { ...form, slug: form.slug || null };
      if (editingId !== null) {
        await updateArticle(editingId, payload);
      } else {
        await createArticle(payload);
      }
      await loadItems();
      mode = 'list';
      toastMessage = editingId ? 'Article updated' : 'Article created';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      saving = false;
    }
  }

  async function handleDelete(id: number) {
    if (!confirm('Delete this article?')) return;
    try {
      await deleteArticle(id);
      await loadItems();
      toastMessage = 'Article deleted';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to delete';
      toastType = 'error';
    }
  }

  function cancel() {
    mode = 'list';
  }

  const cardStyle = 'background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.5rem; padding: 1rem 1.25rem; display: flex; align-items: flex-start; gap: 1rem; margin-bottom: 0.5rem; cursor: pointer; transition: border-color 0.15s, background 0.15s;';
  const cardBodyStyle = 'flex: 1; min-width: 0;';
  const cardTitleStyle = 'font-size: 0.9375rem; font-weight: 500; color: var(--nb-text);';
  const cardMetaStyle = 'font-size: 0.75rem; color: var(--nb-text2); margin-top: 0.1875rem;';
  const cardActionsStyle = 'display: flex; gap: 0.5rem; align-items: center;';
  const actionBtnStyle = 'padding: 0.375rem 0.625rem; background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; color: var(--nb-text3); font-size: 0.75rem; cursor: pointer;';
  const tagPillStyle = 'display: inline-block; font-size: 0.6875rem; padding: 0.125rem 0.4375rem; border-radius: 0.1875rem; background: var(--nb-bg4); border: 1px solid var(--nb-border); color: var(--nb-text2); margin-top: 0.375rem;';
</script>

<Toast bind:message={toastMessage} bind:type={toastType} />

<div style="padding: 1.5rem 2rem 3rem; max-width: 56.25rem;">
  {#if loading}
    <div style="color: var(--nb-text3); font-size: 0.875rem; padding: 1rem 0;">Loading…</div>
  {:else if mode === 'list'}
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.25rem;">
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">Articles</h1>
      <button
        style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.5rem 1rem; color: var(--nb-text2); font-size: 0.8125rem; cursor: pointer;"
        onclick={startCreate}
      >+ Add New</button>
    </div>

    {#each items as item}
      <div style={cardStyle}>
        <div style={cardBodyStyle}>
          <div style={cardTitleStyle}>{item.title}</div>
          <div style={cardMetaStyle}>{item.summary}</div>
          <span style="{tagPillStyle} {item.published ? 'color: #7fd4a8; background: var(--nb-green-dim);' : ''}">{item.published ? 'published' : 'draft'}</span>
        </div>
        <div style={cardActionsStyle}>
          <button style={actionBtnStyle} onclick={() => startEdit(item)}>Edit</button>
          <button style="{actionBtnStyle} color: var(--nb-red-text);" onclick={() => handleDelete(item.id)}>Delete</button>
        </div>
      </div>
    {/each}
  {:else}
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.25rem;">
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">{editingId ? 'Edit Article' : 'New Article'}</h1>
    </div>

    <div style="background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;">
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-title">Title</label>
          <input id="field-title" class="nb-input" bind:value={form.title} />
        </div>
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-slug">Slug</label>
          <input id="field-slug" class="nb-input" bind:value={form.slug} placeholder="Leave blank to auto-generate" />
        </div>
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-summary">Summary</label>
          <textarea id="field-summary" class="nb-input" rows="3" bind:value={form.summary}></textarea>
        </div>
        <div style="grid-column: 1 / -1;">
          <div class="nb-label">Content</div>
          <MarkdownEditor bind:value={form.content} />
        </div>
        <div style="grid-column: 1 / -1;">
          <div class="nb-label">Tags</div>
          <TagInput bind:value={form.tags} />
        </div>
        <div>
          <label class="nb-label" for="field-published-at">Published At</label>
          <input id="field-published-at" type="date" class="nb-input" bind:value={form.published_at} />
        </div>
        <div style="display: flex; align-items: center; gap: 0.5rem; padding-top: 1.25rem;">
          <input type="checkbox" id="field-published" bind:checked={form.published} />
          <label class="nb-label" for="field-published" style="margin-bottom: 0;">Published</label>
        </div>
      </div>
    </div>

    <div style="display: flex; gap: 0.5rem;">
      <button
        style="background: transparent; border: 1px solid var(--nb-border); color: var(--nb-text2); border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; cursor: pointer;"
        onclick={cancel}
      >Cancel</button>
      <button
        style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;"
        onclick={handleSave}
        disabled={saving}
      >{saving ? 'Saving…' : 'Save'}</button>
    </div>
  {/if}
</div>
