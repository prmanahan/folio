<script lang="ts">
  import { onMount } from 'svelte';
  import { listExperience, createExperience, updateExperience, deleteExperience } from '$lib/admin-api';
  import type { ExperienceFull, ExperienceInput } from '$lib/admin-types';
  import FormSection from '$lib/components/admin/FormSection.svelte';
  import ListInput from '$lib/components/admin/ListInput.svelte';
  import Toast from '$lib/components/admin/Toast.svelte';

  let items = $state<ExperienceFull[]>([]);
  let editingId = $state<number | null>(null);
  let creatingNew = $state(false);
  let form = $state<ExperienceInput>(emptyForm());
  let loading = $state(true);
  let saving = $state(false);
  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');

  function emptyForm(): ExperienceInput {
    return {
      company_name: '',
      title: '',
      location: '',
      start_date: '',
      end_date: null,
      is_current: false,
      summary: '',
      bullet_points: [],
      display_order: 0,
      title_progression: '',
      quantified_impact: [],
      why_joined: '',
      why_left: '',
      actual_contributions: '',
      proudest_achievement: '',
      would_do_differently: '',
      challenges_faced: '',
      lessons_learned: '',
      manager_would_say: '',
      reports_would_say: '',
    };
  }

  async function loadItems() {
    items = await listExperience();
  }

  onMount(async () => {
    try {
      await loadItems();
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to load experience';
      toastType = 'error';
    } finally {
      loading = false;
    }
  });

  function startCreate() {
    form = emptyForm();
    editingId = null;
    creatingNew = true;
  }

  function startEdit(item: ExperienceFull) {
    creatingNew = false;
    editingId = item.id;
    const qi = item.quantified_impact;
    form = {
      company_name: item.company_name,
      title: item.title,
      location: item.location,
      start_date: item.start_date,
      end_date: item.end_date,
      is_current: item.is_current,
      summary: item.summary,
      bullet_points: Array.isArray(item.bullet_points) ? item.bullet_points : [],
      display_order: item.display_order,
      title_progression: item.title_progression,
      quantified_impact: Array.isArray(qi) ? qi : [],
      why_joined: item.why_joined,
      why_left: item.why_left,
      actual_contributions: item.actual_contributions,
      proudest_achievement: item.proudest_achievement,
      would_do_differently: item.would_do_differently,
      challenges_faced: item.challenges_faced,
      lessons_learned: item.lessons_learned,
      manager_would_say: item.manager_would_say,
      reports_would_say: item.reports_would_say,
    };
  }

  async function handleSave() {
    saving = true;
    try {
      if (editingId !== null) {
        await updateExperience(editingId, form);
      } else {
        await createExperience(form);
      }
      await loadItems();
      editingId = null;
      creatingNew = false;
      toastMessage = editingId !== null ? 'Experience updated' : 'Experience created';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      saving = false;
    }
  }

  async function handleDelete(id: number) {
    if (!confirm('Delete this experience?')) return;
    try {
      await deleteExperience(id);
      if (editingId === id) {
        editingId = null;
        creatingNew = false;
      }
      await loadItems();
      toastMessage = 'Experience deleted';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to delete';
      toastType = 'error';
    }
  }

  function cancel() {
    editingId = null;
    creatingNew = false;
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
  {:else}
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.25rem;">
      <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">Experience</h1>
      <button
        style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.5rem 1rem; color: var(--nb-text2); font-size: 0.8125rem; cursor: pointer;"
        onclick={startCreate}
      >+ Add New</button>
    </div>

    {#if creatingNew}
      <div style="border: 1px solid var(--nb-gold); border-radius: 0.5rem; background: var(--nb-bg); margin-bottom: 1rem;">
        <div style="padding: 0.875rem 1.25rem; background: var(--nb-bg2); border-bottom: 1px solid var(--nb-border); display: flex; align-items: center; justify-content: space-between; border-radius: 0.5rem 0.5rem 0 0;">
          <span style="font-size: 0.8125rem; font-weight: 600; color: var(--nb-text);">New Experience</span>
          <div style="display: flex; gap: 0.5rem;">
            <button style="padding: 0.375rem 0.75rem; background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; color: var(--nb-text2); font-size: 0.75rem; cursor: pointer;" onclick={cancel}>Cancel</button>
            <button style="padding: 0.375rem 0.75rem; background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; font-size: 0.75rem; font-weight: 600; cursor: pointer;" onclick={handleSave} disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
          </div>
        </div>
        <div style="padding: 1.25rem;">
          <FormSection title="Public Information" tier="public">
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
              <div>
                <label class="nb-label" for="new-company-name">Company Name</label>
                <input id="new-company-name" class="nb-input" bind:value={form.company_name} />
              </div>
              <div>
                <label class="nb-label" for="new-title">Title</label>
                <input id="new-title" class="nb-input" bind:value={form.title} />
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-location">Location</label>
                <input id="new-location" class="nb-input" bind:value={form.location} />
              </div>
              <div>
                <label class="nb-label" for="new-start-date">Start Date</label>
                <input id="new-start-date" class="nb-input" placeholder="e.g. 2020-01" bind:value={form.start_date} />
              </div>
              <div>
                <label class="nb-label" for="new-end-date">End Date</label>
                <input id="new-end-date" class="nb-input" placeholder="e.g. 2023-06" bind:value={form.end_date} disabled={form.is_current} />
              </div>
              <div style="grid-column: 1 / -1; display: flex; align-items: center; gap: 0.5rem;">
                <input type="checkbox" id="new-is-current" bind:checked={form.is_current} />
                <label class="nb-label" for="new-is-current" style="margin-bottom: 0;">Currently working here</label>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-summary">Summary</label>
                <textarea id="new-summary" class="nb-input" rows="4" bind:value={form.summary}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <div class="nb-label">Bullet Points</div>
                <ListInput bind:value={form.bullet_points} placeholder="Add bullet point" />
              </div>
              <div>
                <label class="nb-label" for="new-display-order">Display Order</label>
                <input id="new-display-order" type="number" class="nb-input" bind:value={form.display_order} />
              </div>
            </div>
          </FormSection>
          <FormSection title="AI Context" tier="ai">
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-title-progression">Title Progression</label>
                <input id="new-title-progression" class="nb-input" bind:value={form.title_progression} />
              </div>
              <div style="grid-column: 1 / -1;">
                <div class="nb-label">Quantified Impact</div>
                <ListInput bind:value={form.quantified_impact} placeholder="Add impact metric" />
              </div>
            </div>
          </FormSection>
          <FormSection title="Private Notes" tier="private" collapsed>
            <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem; margin-top: 0.75rem;">
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-why-joined">Why Joined</label>
                <textarea id="new-why-joined" class="nb-input" rows="3" bind:value={form.why_joined}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-why-left">Why Left</label>
                <textarea id="new-why-left" class="nb-input" rows="3" bind:value={form.why_left}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-actual-contributions">Actual Contributions</label>
                <textarea id="new-actual-contributions" class="nb-input" rows="3" bind:value={form.actual_contributions}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-proudest-achievement">Proudest Achievement</label>
                <textarea id="new-proudest-achievement" class="nb-input" rows="3" bind:value={form.proudest_achievement}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-would-do-differently">Would Do Differently</label>
                <textarea id="new-would-do-differently" class="nb-input" rows="3" bind:value={form.would_do_differently}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-challenges-faced">Challenges Faced</label>
                <textarea id="new-challenges-faced" class="nb-input" rows="3" bind:value={form.challenges_faced}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-lessons-learned">Lessons Learned</label>
                <textarea id="new-lessons-learned" class="nb-input" rows="3" bind:value={form.lessons_learned}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-manager-would-say">Manager Would Say</label>
                <textarea id="new-manager-would-say" class="nb-input" rows="3" bind:value={form.manager_would_say}></textarea>
              </div>
              <div style="grid-column: 1 / -1;">
                <label class="nb-label" for="new-reports-would-say">Reports Would Say</label>
                <textarea id="new-reports-would-say" class="nb-input" rows="3" bind:value={form.reports_would_say}></textarea>
              </div>
            </div>
          </FormSection>
        </div>
        <div style="padding: 0.875rem 1.25rem; background: var(--nb-bg2); border-top: 1px solid var(--nb-border); display: flex; align-items: center; border-radius: 0 0 0.5rem 0.5rem;">
          <button style="padding: 0.375rem 0.75rem; background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; font-size: 0.75rem; font-weight: 600; cursor: pointer; margin-right: 0.5rem;" onclick={handleSave} disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
          <button style="padding: 0.375rem 0.75rem; background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; color: var(--nb-text2); font-size: 0.75rem; cursor: pointer;" onclick={cancel}>Discard</button>
        </div>
      </div>
    {/if}

    {#each items as item}
      <div>
        <div style={cardStyle}>
          <div style={cardBodyStyle}>
            <div style={cardTitleStyle}>{item.title}</div>
            <div style={cardMetaStyle}>{item.company_name} · {item.start_date} — {item.is_current ? 'Present' : (item.end_date ?? '')}</div>
            {#if item.is_current}
              <span style={tagPillStyle}>current</span>
            {/if}
          </div>
          <div style={cardActionsStyle}>
            <button
              style={actionBtnStyle}
              onclick={() => editingId === item.id ? cancel() : startEdit(item)}
            >{editingId === item.id ? 'Close' : 'Edit'}</button>
            <button
              style="{actionBtnStyle} color: var(--nb-red-text);"
              onclick={() => handleDelete(item.id)}
            >Delete</button>
          </div>
        </div>

        {#if editingId === item.id}
          <div style="border: 1px solid var(--nb-gold); border-radius: 0.5rem; background: var(--nb-bg); margin-bottom: 0.5rem; margin-top: -0.25rem;">
            <div style="padding: 0.875rem 1.25rem; background: var(--nb-bg2); border-bottom: 1px solid var(--nb-border); display: flex; align-items: center; justify-content: space-between; border-radius: 0.5rem 0.5rem 0 0;">
              <span style="font-size: 0.8125rem; font-weight: 600; color: var(--nb-text);">Editing: {item.company_name}</span>
              <div style="display: flex; gap: 0.5rem;">
                <button style="padding: 0.375rem 0.75rem; background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; color: var(--nb-text2); font-size: 0.75rem; cursor: pointer;" onclick={cancel}>Cancel</button>
                <button style="padding: 0.375rem 0.75rem; background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; font-size: 0.75rem; font-weight: 600; cursor: pointer;" onclick={handleSave} disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
              </div>
            </div>
            <div style="padding: 1.25rem;">
              <FormSection title="Public Information" tier="public">
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
                  <div>
                    <label class="nb-label" for="edit-company-name-{item.id}">Company Name</label>
                    <input id="edit-company-name-{item.id}" class="nb-input" bind:value={form.company_name} />
                  </div>
                  <div>
                    <label class="nb-label" for="edit-title-{item.id}">Title</label>
                    <input id="edit-title-{item.id}" class="nb-input" bind:value={form.title} />
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-location-{item.id}">Location</label>
                    <input id="edit-location-{item.id}" class="nb-input" bind:value={form.location} />
                  </div>
                  <div>
                    <label class="nb-label" for="edit-start-date-{item.id}">Start Date</label>
                    <input id="edit-start-date-{item.id}" class="nb-input" placeholder="e.g. 2020-01" bind:value={form.start_date} />
                  </div>
                  <div>
                    <label class="nb-label" for="edit-end-date-{item.id}">End Date</label>
                    <input id="edit-end-date-{item.id}" class="nb-input" placeholder="e.g. 2023-06" bind:value={form.end_date} disabled={form.is_current} />
                  </div>
                  <div style="grid-column: 1 / -1; display: flex; align-items: center; gap: 0.5rem;">
                    <input type="checkbox" id="edit-is-current-{item.id}" bind:checked={form.is_current} />
                    <label class="nb-label" for="edit-is-current-{item.id}" style="margin-bottom: 0;">Currently working here</label>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-summary-{item.id}">Summary</label>
                    <textarea id="edit-summary-{item.id}" class="nb-input" rows="4" bind:value={form.summary}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <div class="nb-label">Bullet Points</div>
                    <ListInput bind:value={form.bullet_points} placeholder="Add bullet point" />
                  </div>
                  <div>
                    <label class="nb-label" for="edit-display-order-{item.id}">Display Order</label>
                    <input id="edit-display-order-{item.id}" type="number" class="nb-input" bind:value={form.display_order} />
                  </div>
                </div>
              </FormSection>
              <FormSection title="AI Context" tier="ai">
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-title-progression-{item.id}">Title Progression</label>
                    <input id="edit-title-progression-{item.id}" class="nb-input" bind:value={form.title_progression} />
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <div class="nb-label">Quantified Impact</div>
                    <ListInput bind:value={form.quantified_impact} placeholder="Add impact metric" />
                  </div>
                </div>
              </FormSection>
              <FormSection title="Private Notes" tier="private" collapsed>
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem; margin-top: 0.75rem;">
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-why-joined-{item.id}">Why Joined</label>
                    <textarea id="edit-why-joined-{item.id}" class="nb-input" rows="3" bind:value={form.why_joined}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-why-left-{item.id}">Why Left</label>
                    <textarea id="edit-why-left-{item.id}" class="nb-input" rows="3" bind:value={form.why_left}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-actual-contributions-{item.id}">Actual Contributions</label>
                    <textarea id="edit-actual-contributions-{item.id}" class="nb-input" rows="3" bind:value={form.actual_contributions}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-proudest-achievement-{item.id}">Proudest Achievement</label>
                    <textarea id="edit-proudest-achievement-{item.id}" class="nb-input" rows="3" bind:value={form.proudest_achievement}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-would-do-differently-{item.id}">Would Do Differently</label>
                    <textarea id="edit-would-do-differently-{item.id}" class="nb-input" rows="3" bind:value={form.would_do_differently}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-challenges-faced-{item.id}">Challenges Faced</label>
                    <textarea id="edit-challenges-faced-{item.id}" class="nb-input" rows="3" bind:value={form.challenges_faced}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-lessons-learned-{item.id}">Lessons Learned</label>
                    <textarea id="edit-lessons-learned-{item.id}" class="nb-input" rows="3" bind:value={form.lessons_learned}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-manager-would-say-{item.id}">Manager Would Say</label>
                    <textarea id="edit-manager-would-say-{item.id}" class="nb-input" rows="3" bind:value={form.manager_would_say}></textarea>
                  </div>
                  <div style="grid-column: 1 / -1;">
                    <label class="nb-label" for="edit-reports-would-say-{item.id}">Reports Would Say</label>
                    <textarea id="edit-reports-would-say-{item.id}" class="nb-input" rows="3" bind:value={form.reports_would_say}></textarea>
                  </div>
                </div>
              </FormSection>
            </div>
            <div style="padding: 0.875rem 1.25rem; background: var(--nb-bg2); border-top: 1px solid var(--nb-border); display: flex; align-items: center; border-radius: 0 0 0.5rem 0.5rem;">
              <button style="padding: 0.375rem 0.75rem; background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; font-size: 0.75rem; font-weight: 600; cursor: pointer; margin-right: 0.5rem;" onclick={handleSave} disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
              <button style="padding: 0.375rem 0.75rem; background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; color: var(--nb-text2); font-size: 0.75rem; cursor: pointer;" onclick={cancel}>Discard</button>
            </div>
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>
