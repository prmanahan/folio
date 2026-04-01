<script lang="ts">
  import { onMount } from 'svelte';
  import { listSkills, createSkill, updateSkill, deleteSkill } from '$lib/admin-api';
  import type { SkillFull, SkillInput } from '$lib/admin-types';
  import FormSection from '$lib/components/admin/FormSection.svelte';
  import Toast from '$lib/components/admin/Toast.svelte';

  let items = $state<SkillFull[]>([]);
  let mode = $state<'list' | 'edit'>('list');
  let editingId = $state<number | null>(null);
  let form = $state<SkillInput>(emptyForm());
  let loading = $state(true);
  let saving = $state(false);
  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');
  let isDirty = $state(false);
  let deletingId = $state<number | null>(null);

  function emptyForm(): SkillInput {
    return {
      skill_name: '', category: '', years_experience: 0,
      last_used: '', self_rating: 3, evidence: '', honest_notes: '',
    };
  }

  async function loadItems() {
    items = await listSkills();
  }

  onMount(async () => {
    try {
      await loadItems();
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to load skills';
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

  function startEdit(item: SkillFull) {
    editingId = item.id;
    form = {
      skill_name: item.skill_name,
      category: item.category,
      years_experience: item.years_experience,
      last_used: item.last_used,
      self_rating: item.self_rating,
      evidence: item.evidence,
      honest_notes: item.honest_notes,
    };
    isDirty = false;
    mode = 'edit';
  }

  async function handleSave() {
    saving = true;
    try {
      if (editingId !== null) {
        await updateSkill(editingId, form);
      } else {
        await createSkill(form);
      }
      await loadItems();
      mode = 'list';
      isDirty = false;
      toastMessage = editingId ? 'Skill updated' : 'Skill created';
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
      await deleteSkill(id);
      deletingId = null;
      await loadItems();
      toastMessage = 'Skill deleted';
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
  const tagPillStyle = 'display: inline-block; font-size: 0.6875rem; padding: 0.125rem 0.4375rem; border-radius: 0.1875rem; background: var(--nb-bg4); border: 1px solid var(--nb-border); color: var(--nb-text2); margin-top: 0.375rem; margin-right: 0.25rem;';
</script>

<Toast bind:message={toastMessage} bind:type={toastType} />

<div style="padding: 1.5rem 2rem 3rem; max-width: 56.25rem;">
  {#if loading}
    <div style="color: var(--nb-text3); font-size: 0.875rem; padding: 1rem 0;">Loading…</div>
  {:else if mode === 'list'}
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.25rem;">
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">Skills</h1>
      <button
        style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.5rem 1rem; color: var(--nb-text2); font-size: 0.8125rem; cursor: pointer;"
        onclick={startCreate}
      >+ Add New</button>
    </div>

    {#each items as item}
      <div style={cardStyle}>
        <div style={cardBodyStyle}>
          <div style={cardTitleStyle}>{item.skill_name}</div>
          <div style={cardMetaStyle}>{item.category} · {item.years_experience} yr{item.years_experience !== 1 ? 's' : ''} · Rating: {item.self_rating}/5</div>
          {#if item.category}
            <span style={tagPillStyle}>{item.category}</span>
          {/if}
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
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">{editingId ? 'Edit Skill' : 'New Skill'}</h1>
    </div>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div style="background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;" oninput={() => isDirty = true}>
      <FormSection title="Public Information" tier="public">
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
          <div>
            <label class="nb-label" for="field-skill-name">Skill Name</label>
            <input id="field-skill-name" class="nb-input" bind:value={form.skill_name} />
          </div>
          <div>
            <label class="nb-label" for="field-category">Category</label>
            <input id="field-category" class="nb-input" bind:value={form.category} />
          </div>
          <div>
            <label class="nb-label" for="field-years-experience">Years Experience</label>
            <input id="field-years-experience" type="number" class="nb-input" bind:value={form.years_experience} />
          </div>
          <div>
            <label class="nb-label" for="field-last-used">Last Used</label>
            <input id="field-last-used" class="nb-input" bind:value={form.last_used} />
          </div>
        </div>
      </FormSection>

      <FormSection title="AI Context" tier="ai">
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-self-rating">Self Rating: {form.self_rating}/5</label>
            <input id="field-self-rating" type="range" min="1" max="5" class="nb-input" style="padding: 0; height: auto; accent-color: var(--nb-gold);" bind:value={form.self_rating} />
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-evidence">Evidence</label>
            <textarea id="field-evidence" class="nb-input" rows="4" bind:value={form.evidence}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-honest-notes">Honest Notes</label>
            <textarea id="field-honest-notes" class="nb-input" rows="4" bind:value={form.honest_notes}></textarea>
          </div>
        </div>
      </FormSection>
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
