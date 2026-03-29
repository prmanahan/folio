<script lang="ts">
  import { onMount } from 'svelte';
  import { listAgents, createAgent, updateAgent, deleteAgent } from '$lib/admin-api';
  import type { AgentFull, AgentInput } from '$lib/admin-types';
  import Toast from '$lib/components/admin/Toast.svelte';
  import ListInput from '$lib/components/admin/ListInput.svelte';

  let items = $state<AgentFull[]>([]);
  let mode = $state<'list' | 'edit'>('list');
  let editingId = $state<number | null>(null);
  let form = $state<AgentInput>(emptyForm());
  let loading = $state(true);
  let saving = $state(false);
  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');

  function emptyForm(): AgentInput {
    return {
      name: '',
      role: '',
      short_role: '',
      model: 'sonnet',
      personality_blurb: '',
      responsibilities: [],
      avatar_filename: '',
      display_order: 0,
      is_featured: false,
      is_review_gate: false,
      published: true,
    };
  }

  async function loadItems() {
    items = await listAgents();
  }

  onMount(async () => {
    try {
      await loadItems();
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to load agents';
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

  function startEdit(item: AgentFull) {
    editingId = item.id;
    form = {
      name: item.name,
      role: item.role,
      short_role: item.short_role,
      model: item.model,
      personality_blurb: item.personality_blurb,
      responsibilities: [...item.responsibilities],
      avatar_filename: item.avatar_filename,
      display_order: item.display_order,
      is_featured: item.is_featured,
      is_review_gate: item.is_review_gate,
      published: item.published,
    };
    mode = 'edit';
  }

  async function handleSave() {
    saving = true;
    try {
      if (editingId !== null) {
        await updateAgent(editingId, form);
      } else {
        await createAgent(form);
      }
      await loadItems();
      mode = 'list';
      toastMessage = editingId ? 'Agent updated' : 'Agent created';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      saving = false;
    }
  }

  async function handleDelete(id: number) {
    if (!confirm('Delete this agent?')) return;
    try {
      await deleteAgent(id);
      await loadItems();
      toastMessage = 'Agent deleted';
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
  const tagPillStyle = 'display: inline-block; font-size: 0.6875rem; padding: 0.125rem 0.4375rem; border-radius: 0.1875rem; background: var(--nb-bg4); border: 1px solid var(--nb-border); color: var(--nb-text2); margin-top: 0.375rem; margin-right: 0.25rem;';
</script>

<Toast bind:message={toastMessage} bind:type={toastType} />

<div style="padding: 1.5rem 2rem 3rem; max-width: 56.25rem;">
  {#if loading}
    <div style="color: var(--nb-text3); font-size: 0.875rem; padding: 1rem 0;">Loading…</div>
  {:else if mode === 'list'}
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.25rem;">
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">Agents</h1>
      <button
        style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.5rem 1rem; color: var(--nb-text2); font-size: 0.8125rem; cursor: pointer;"
        onclick={startCreate}
      >+ Add New</button>
    </div>

    {#each items as item}
      <div style={cardStyle}>
        <div style={cardBodyStyle}>
          <div style={cardTitleStyle}>{item.name}</div>
          <div style={cardMetaStyle}>{item.short_role} · {item.model}</div>
          {#if !item.published}
            <span style={tagPillStyle}>unpublished</span>
          {:else}
            <span style="{tagPillStyle} color: #7fd4a8; background: var(--nb-green-dim);">published</span>
          {/if}
          {#if item.is_featured}
            <span style={tagPillStyle}>featured</span>
          {/if}
          {#if item.is_review_gate}
            <span style={tagPillStyle}>review gate</span>
          {/if}
        </div>
        <div style={cardActionsStyle}>
          <button style={actionBtnStyle} onclick={() => startEdit(item)}>Edit</button>
          <button style="{actionBtnStyle} color: var(--nb-red-text);" onclick={() => handleDelete(item.id)}>Delete</button>
        </div>
      </div>
    {/each}
  {:else}
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.25rem;">
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">{editingId ? 'Edit Agent' : 'New Agent'}</h1>
    </div>

    <div style="background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;">
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
        <div>
          <label class="nb-label" for="field-name">Name</label>
          <input id="field-name" class="nb-input" bind:value={form.name} />
        </div>
        <div>
          <label class="nb-label" for="field-short-role">Short Role</label>
          <input id="field-short-role" class="nb-input" bind:value={form.short_role} />
        </div>
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-role">Role</label>
          <input id="field-role" class="nb-input" bind:value={form.role} />
        </div>
        <div>
          <label class="nb-label" for="field-model">Model</label>
          <select id="field-model" class="nb-input" bind:value={form.model}>
            <option value="sonnet">Sonnet</option>
            <option value="opus">Opus</option>
          </select>
        </div>
        <div>
          <label class="nb-label" for="field-order">Display Order</label>
          <input id="field-order" type="number" class="nb-input" bind:value={form.display_order} />
        </div>
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-blurb">Personality Blurb</label>
          <textarea id="field-blurb" class="nb-input" rows="3" bind:value={form.personality_blurb}></textarea>
        </div>
        <div style="grid-column: 1 / -1;">
          <div class="nb-label">Responsibilities</div>
          <ListInput bind:value={form.responsibilities} placeholder="Enter responsibility" />
        </div>
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-avatar">Avatar Filename</label>
          <input id="field-avatar" class="nb-input" bind:value={form.avatar_filename} />
        </div>
        <div style="grid-column: 1 / -1; display: flex; flex-direction: column; gap: 0.5rem;">
          <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
            <input type="checkbox" bind:checked={form.is_featured} />
            <span class="nb-label" style="margin-bottom: 0;">Featured (shows as hero band)</span>
          </label>
          <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
            <input type="checkbox" bind:checked={form.is_review_gate} />
            <span class="nb-label" style="margin-bottom: 0;">Review Gate</span>
          </label>
          <label style="display: flex; align-items: center; gap: 0.5rem; cursor: pointer;">
            <input type="checkbox" bind:checked={form.published} />
            <span class="nb-label" style="margin-bottom: 0;">Published</span>
          </label>
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
