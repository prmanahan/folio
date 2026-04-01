<script lang="ts">
  import { onMount } from 'svelte';
  import { listEducation, createEducation, updateEducation, deleteEducation } from '$lib/admin-api';
  import type { Education, EducationInput } from '$lib/admin-types';
  import Toast from '$lib/components/admin/Toast.svelte';

  let items = $state<Education[]>([]);
  let mode = $state<'list' | 'edit'>('list');
  let editingId = $state<number | null>(null);
  let form = $state<EducationInput>(emptyForm());
  let loading = $state(true);
  let saving = $state(false);
  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');
  let isDirty = $state(false);
  let deletingId = $state<number | null>(null);

  function emptyForm(): EducationInput {
    return {
      degree: '',
      institution: '',
      location: '',
      start_year: '',
      end_year: '',
    };
  }

  async function loadItems() {
    items = await listEducation();
  }

  onMount(async () => {
    try {
      await loadItems();
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to load education';
      toastType = 'error';
    } finally {
      loading = false;
    }
  });

  function startCreate() {
    form = emptyForm();
    editingId = null;
    isDirty = false;
    mode = 'edit';
  }

  function startEdit(item: Education) {
    editingId = item.id;
    form = {
      degree: item.degree,
      institution: item.institution,
      location: item.location,
      start_year: item.start_year,
      end_year: item.end_year,
    };
    isDirty = false;
    mode = 'edit';
  }

  async function handleSave() {
    saving = true;
    try {
      if (editingId !== null) {
        await updateEducation(editingId, form);
      } else {
        await createEducation(form);
      }
      await loadItems();
      mode = 'list';
      isDirty = false;
      toastMessage = editingId ? 'Education updated' : 'Education created';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      saving = false;
    }
  }

  async function handleDelete(id: number) {
    try {
      await deleteEducation(id);
      deletingId = null;
      await loadItems();
      toastMessage = 'Education deleted';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to delete';
      toastType = 'error';
    }
  }

  function cancel() {
    mode = 'list';
    isDirty = false;
  }

  const cardStyle = 'background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.5rem; padding: 1rem 1.25rem; display: flex; align-items: flex-start; gap: 1rem; margin-bottom: 0.5rem; cursor: pointer; transition: border-color 0.15s, background 0.15s;';
  const cardBodyStyle = 'flex: 1; min-width: 0;';
  const cardTitleStyle = 'font-size: 0.9375rem; font-weight: 500; color: var(--nb-text);';
  const cardMetaStyle = 'font-size: 0.75rem; color: var(--nb-text2); margin-top: 0.1875rem;';
  const cardActionsStyle = 'display: flex; gap: 0.5rem; align-items: center;';
  const actionBtnStyle = 'padding: 0.375rem 0.625rem; background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; color: var(--nb-text3); font-size: 0.75rem; cursor: pointer;';
</script>

<Toast bind:message={toastMessage} bind:type={toastType} />

<div style="padding: 1.5rem 2rem 3rem; max-width: 56.25rem;">
  {#if loading}
    <div style="color: var(--nb-text3); font-size: 0.875rem; padding: 1rem 0;">Loading…</div>
  {:else if mode === 'list'}
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.25rem;">
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">Education</h1>
      <button
        style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.5rem 1rem; color: var(--nb-text2); font-size: 0.8125rem; cursor: pointer;"
        onclick={startCreate}
      >+ Add New</button>
    </div>

    {#each items as item}
      <div style={cardStyle}>
        <div style={cardBodyStyle}>
          <div style={cardTitleStyle}>{item.degree}</div>
          <div style={cardMetaStyle}>{item.institution} · {item.start_year} — {item.end_year}</div>
        </div>
        <div style={cardActionsStyle}>
          {#if deletingId === item.id}
            <span style="font-size: 0.75rem; color: var(--nb-text2);">Are you sure?</span>
            <button
              style="{actionBtnStyle} border-color: var(--nb-red); color: var(--nb-red-text);"
              onclick={() => handleDelete(item.id)}
            >Yes, delete</button>
            <button
              style={actionBtnStyle}
              onclick={() => deletingId = null}
            >Cancel</button>
          {:else}
            <button style={actionBtnStyle} onclick={() => startEdit(item)}>Edit</button>
            <button style="{actionBtnStyle} color: var(--nb-red-text);" onclick={() => deletingId = item.id}>Delete</button>
          {/if}
        </div>
      </div>
    {/each}
  {:else}
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.25rem;">
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">{editingId ? 'Edit Education' : 'New Education'}</h1>
    </div>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div style="background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;" oninput={() => isDirty = true}>
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-degree">Degree</label>
          <input id="field-degree" class="nb-input" bind:value={form.degree} />
        </div>
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-institution">Institution</label>
          <input id="field-institution" class="nb-input" bind:value={form.institution} />
        </div>
        <div style="grid-column: 1 / -1;">
          <label class="nb-label" for="field-location">Location</label>
          <input id="field-location" class="nb-input" bind:value={form.location} />
        </div>
        <div>
          <label class="nb-label" for="field-start-year">Start Year</label>
          <input id="field-start-year" class="nb-input" bind:value={form.start_year} />
        </div>
        <div>
          <label class="nb-label" for="field-end-year">End Year</label>
          <input id="field-end-year" class="nb-input" bind:value={form.end_year} />
        </div>
      </div>
    </div>

    <div style="display: flex; gap: 0.5rem; align-items: center;">
      <button
        style="background: transparent; border: 1px solid var(--nb-border); color: var(--nb-text2); border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; cursor: pointer;"
        onclick={cancel}
      >Cancel</button>
      <button
        style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;"
        onclick={handleSave}
        disabled={saving}
      >{saving ? 'Saving…' : 'Save'}</button>
      {#if isDirty}
        <span style="margin-left: auto; display: flex; align-items: center; gap: 0.375rem;">
          <span style="width: 6px; height: 6px; border-radius: 50%; background: var(--nb-amber);"></span>
          <span style="font-size: 0.6875rem; color: var(--nb-text3);">Unsaved changes</span>
        </span>
      {/if}
    </div>
  {/if}
</div>
