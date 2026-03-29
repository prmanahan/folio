<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getValues, updateValues,
    listGaps, createGap, updateGap, deleteGap,
    listFaq, createFaq, updateFaq, deleteFaq,
    listInstructions, createInstruction, updateInstruction, deleteInstruction,
  } from '$lib/admin-api';
  import type {
    ValuesCultureInput,
    GapWeakness, GapWeaknessInput,
    FaqFull, FaqInput,
    AiInstruction, AiInstructionInput,
  } from '$lib/admin-types';
  import Toast from '$lib/components/admin/Toast.svelte';
  import MarkdownEditor from '$lib/components/admin/MarkdownEditor.svelte';

  let activeTab = $state<'values' | 'gaps' | 'faq' | 'instructions'>('values');

  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');

  // --- Values tab state ---
  let valuesForm = $state<ValuesCultureInput>({
    must_haves: '',
    dealbreakers: '',
    management_style_preferences: '',
    team_size_preferences: '',
    how_handle_conflict: '',
    how_handle_ambiguity: '',
    how_handle_failure: '',
  });
  let valuesLoading = $state(true);
  let valuesSaving = $state(false);

  // --- Gaps tab state ---
  let gapItems = $state<GapWeakness[]>([]);
  let gapMode = $state<'list' | 'edit'>('list');
  let gapEditingId = $state<number | null>(null);
  let gapForm = $state<GapWeaknessInput>(emptyGapForm());
  let gapSaving = $state(false);

  function emptyGapForm(): GapWeaknessInput {
    return { gap_type: 'skill', description: '', why_its_a_gap: '', interest_in_learning: false };
  }

  // --- FAQ tab state ---
  let faqItems = $state<FaqFull[]>([]);
  let faqMode = $state<'list' | 'edit'>('list');
  let faqEditingId = $state<number | null>(null);
  let faqForm = $state<FaqInput>(emptyFaqForm());
  let faqSaving = $state(false);

  function emptyFaqForm(): FaqInput {
    return { question: '', answer: '', is_common_question: false };
  }

  // --- Instructions tab state ---
  let instrItems = $state<AiInstruction[]>([]);
  let instrMode = $state<'list' | 'edit'>('list');
  let instrEditingId = $state<number | null>(null);
  let instrForm = $state<AiInstructionInput>(emptyInstrForm());
  let instrSaving = $state(false);

  function emptyInstrForm(): AiInstructionInput {
    return { instruction_type: '', instruction: '', priority: 0 };
  }

  // --- Load all data on mount ---
  onMount(async () => {
    try {
      const [vals, gaps, faqs, instrs] = await Promise.all([
        getValues(), listGaps(), listFaq(), listInstructions()
      ]);
      valuesForm = {
        must_haves: vals.must_haves,
        dealbreakers: vals.dealbreakers,
        management_style_preferences: vals.management_style_preferences,
        team_size_preferences: vals.team_size_preferences,
        how_handle_conflict: vals.how_handle_conflict,
        how_handle_ambiguity: vals.how_handle_ambiguity,
        how_handle_failure: vals.how_handle_failure,
      };
      gapItems = gaps;
      faqItems = faqs;
      instrItems = instrs;
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to load data';
      toastType = 'error';
    } finally {
      valuesLoading = false;
    }
  });

  // --- Values handlers ---
  async function handleValuesSave() {
    valuesSaving = true;
    try {
      await updateValues(valuesForm);
      toastMessage = 'Values & Culture saved';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      valuesSaving = false;
    }
  }

  // --- Gap handlers ---
  function startGapCreate() {
    gapForm = emptyGapForm();
    gapEditingId = null;
    gapMode = 'edit';
  }

  function startGapEdit(item: GapWeakness) {
    gapEditingId = item.id;
    gapForm = {
      gap_type: item.gap_type,
      description: item.description,
      why_its_a_gap: item.why_its_a_gap,
      interest_in_learning: item.interest_in_learning,
    };
    gapMode = 'edit';
  }

  async function handleGapSave() {
    gapSaving = true;
    try {
      if (gapEditingId !== null) {
        await updateGap(gapEditingId, gapForm);
      } else {
        await createGap(gapForm);
      }
      gapItems = await listGaps();
      gapMode = 'list';
      toastMessage = gapEditingId ? 'Gap updated' : 'Gap created';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      gapSaving = false;
    }
  }

  async function handleGapDelete(id: number) {
    if (!confirm('Delete this gap?')) return;
    try {
      await deleteGap(id);
      gapItems = await listGaps();
      toastMessage = 'Gap deleted';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to delete';
      toastType = 'error';
    }
  }

  function cancelGap() {
    gapMode = 'list';
  }

  // --- FAQ handlers ---
  function startFaqCreate() {
    faqForm = emptyFaqForm();
    faqEditingId = null;
    faqMode = 'edit';
  }

  function startFaqEdit(item: FaqFull) {
    faqEditingId = item.id;
    faqForm = {
      question: item.question,
      answer: item.answer,
      is_common_question: item.is_common_question,
    };
    faqMode = 'edit';
  }

  async function handleFaqSave() {
    faqSaving = true;
    try {
      if (faqEditingId !== null) {
        await updateFaq(faqEditingId, faqForm);
      } else {
        await createFaq(faqForm);
      }
      faqItems = await listFaq();
      faqMode = 'list';
      toastMessage = faqEditingId ? 'FAQ updated' : 'FAQ created';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      faqSaving = false;
    }
  }

  async function handleFaqDelete(id: number) {
    if (!confirm('Delete this FAQ?')) return;
    try {
      await deleteFaq(id);
      faqItems = await listFaq();
      toastMessage = 'FAQ deleted';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to delete';
      toastType = 'error';
    }
  }

  function cancelFaq() {
    faqMode = 'list';
  }

  // --- Instruction handlers ---
  function startInstrCreate() {
    instrForm = emptyInstrForm();
    instrEditingId = null;
    instrMode = 'edit';
  }

  function startInstrEdit(item: AiInstruction) {
    instrEditingId = item.id;
    instrForm = {
      instruction_type: item.instruction_type,
      instruction: item.instruction,
      priority: item.priority,
    };
    instrMode = 'edit';
  }

  async function handleInstrSave() {
    instrSaving = true;
    try {
      if (instrEditingId !== null) {
        await updateInstruction(instrEditingId, instrForm);
      } else {
        await createInstruction(instrForm);
      }
      instrItems = await listInstructions();
      instrMode = 'list';
      toastMessage = instrEditingId ? 'Instruction updated' : 'Instruction created';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      instrSaving = false;
    }
  }

  async function handleInstrDelete(id: number) {
    if (!confirm('Delete this instruction?')) return;
    try {
      await deleteInstruction(id);
      instrItems = await listInstructions();
      toastMessage = 'Instruction deleted';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to delete';
      toastType = 'error';
    }
  }

  function cancelInstr() {
    instrMode = 'list';
  }

  const tabs: { key: 'values' | 'gaps' | 'faq' | 'instructions'; label: string }[] = [
    { key: 'values', label: 'Values & Culture' },
    { key: 'gaps', label: 'Gaps & Weaknesses' },
    { key: 'faq', label: 'FAQ Responses' },
    { key: 'instructions', label: 'AI Instructions' },
  ];

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
  <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em; margin-bottom: 1.25rem;">AI Config</h1>

  {#if valuesLoading}
    <div style="color: var(--nb-text3); font-size: 0.875rem; padding: 1rem 0;">Loading…</div>
  {:else}
    <!-- Notebook-style tab strip -->
    <div style="display: flex; gap: 0.25rem; margin-bottom: 1.5rem; border-bottom: 1px solid var(--nb-border); padding-bottom: 0;">
      {#each tabs as tab}
        <button
          style="padding: 0.5rem 1rem; background: transparent; border: none; cursor: pointer; font-size: 0.8125rem; font-family: inherit; border-bottom: 2px solid {activeTab === tab.key ? 'var(--nb-gold)' : 'transparent'}; color: {activeTab === tab.key ? 'var(--nb-gold)' : 'var(--nb-text2)'}; font-weight: {activeTab === tab.key ? '500' : '400'}; margin-bottom: -1px;"
          onclick={() => activeTab = tab.key}
        >{tab.label}</button>
      {/each}
    </div>

    {#if activeTab === 'values'}
      <div style="background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;">
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-must-haves">Must Haves</label>
            <textarea id="field-must-haves" class="nb-input" rows="4" bind:value={valuesForm.must_haves}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-dealbreakers">Dealbreakers</label>
            <textarea id="field-dealbreakers" class="nb-input" rows="4" bind:value={valuesForm.dealbreakers}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-management-style-prefs">Management Style Preferences</label>
            <textarea id="field-management-style-prefs" class="nb-input" rows="4" bind:value={valuesForm.management_style_preferences}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-team-size-prefs">Team Size Preferences</label>
            <textarea id="field-team-size-prefs" class="nb-input" rows="3" bind:value={valuesForm.team_size_preferences}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-how-handle-conflict">How I Handle Conflict</label>
            <textarea id="field-how-handle-conflict" class="nb-input" rows="4" bind:value={valuesForm.how_handle_conflict}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-how-handle-ambiguity">How I Handle Ambiguity</label>
            <textarea id="field-how-handle-ambiguity" class="nb-input" rows="4" bind:value={valuesForm.how_handle_ambiguity}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-how-handle-failure">How I Handle Failure</label>
            <textarea id="field-how-handle-failure" class="nb-input" rows="4" bind:value={valuesForm.how_handle_failure}></textarea>
          </div>
        </div>
      </div>
      <div>
        <button
          style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;"
          onclick={handleValuesSave}
          disabled={valuesSaving}
        >{valuesSaving ? 'Saving…' : 'Save'}</button>
      </div>

    {:else if activeTab === 'gaps'}
      {#if gapMode === 'list'}
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
          <span style="font-size: 0.875rem; font-weight: 500; color: var(--nb-text2);">Gaps & Weaknesses</span>
          <button
            style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.5rem 1rem; color: var(--nb-text2); font-size: 0.8125rem; cursor: pointer;"
            onclick={startGapCreate}
          >+ Add New</button>
        </div>
        {#each gapItems as item}
          <div style={cardStyle}>
            <div style={cardBodyStyle}>
              <div style={cardTitleStyle}>{item.description.length > 80 ? item.description.slice(0, 80) + '…' : item.description}</div>
              <div style={cardMetaStyle}>{item.gap_type}</div>
              {#if item.interest_in_learning}
                <span style="{tagPillStyle} color: #7fd4a8; background: var(--nb-green-dim);">interested in learning</span>
              {/if}
            </div>
            <div style={cardActionsStyle}>
              <button style={actionBtnStyle} onclick={() => startGapEdit(item)}>Edit</button>
              <button style="{actionBtnStyle} color: var(--nb-red-text);" onclick={() => handleGapDelete(item.id)}>Delete</button>
            </div>
          </div>
        {/each}
      {:else}
        <div style="background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;">
          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
            <div style="grid-column: 1 / -1;">
              <label class="nb-label" for="field-gap-type">Gap Type</label>
              <select id="field-gap-type" class="nb-input" bind:value={gapForm.gap_type}>
                <option value="skill">Skill</option>
                <option value="experience">Experience</option>
                <option value="environment">Environment</option>
                <option value="role_type">Role Type</option>
              </select>
            </div>
            <div style="grid-column: 1 / -1;">
              <label class="nb-label" for="field-gap-description">Description</label>
              <textarea id="field-gap-description" class="nb-input" rows="4" bind:value={gapForm.description}></textarea>
            </div>
            <div style="grid-column: 1 / -1;">
              <label class="nb-label" for="field-why-gap">Why It's a Gap</label>
              <textarea id="field-why-gap" class="nb-input" rows="4" bind:value={gapForm.why_its_a_gap}></textarea>
            </div>
            <div style="grid-column: 1 / -1; display: flex; align-items: center; gap: 0.5rem;">
              <input type="checkbox" id="field-interest-in-learning" bind:checked={gapForm.interest_in_learning} />
              <label class="nb-label" for="field-interest-in-learning" style="margin-bottom: 0;">Interest in Learning</label>
            </div>
          </div>
        </div>
        <div style="display: flex; gap: 0.5rem;">
          <button style="background: transparent; border: 1px solid var(--nb-border); color: var(--nb-text2); border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; cursor: pointer;" onclick={cancelGap}>Cancel</button>
          <button style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;" onclick={handleGapSave} disabled={gapSaving}>{gapSaving ? 'Saving…' : 'Save'}</button>
        </div>
      {/if}

    {:else if activeTab === 'faq'}
      {#if faqMode === 'list'}
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
          <span style="font-size: 0.875rem; font-weight: 500; color: var(--nb-text2);">FAQ Responses</span>
          <button
            style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.5rem 1rem; color: var(--nb-text2); font-size: 0.8125rem; cursor: pointer;"
            onclick={startFaqCreate}
          >+ Add New</button>
        </div>
        {#each faqItems as item}
          <div style={cardStyle}>
            <div style={cardBodyStyle}>
              <div style={cardTitleStyle}>{item.question.length > 100 ? item.question.slice(0, 100) + '…' : item.question}</div>
              {#if item.is_common_question}
                <span style="{tagPillStyle} color: var(--nb-gold); background: var(--nb-bg3);">common</span>
              {/if}
            </div>
            <div style={cardActionsStyle}>
              <button style={actionBtnStyle} onclick={() => startFaqEdit(item)}>Edit</button>
              <button style="{actionBtnStyle} color: var(--nb-red-text);" onclick={() => handleFaqDelete(item.id)}>Delete</button>
            </div>
          </div>
        {/each}
      {:else}
        <div style="background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;">
          <div style="display: grid; grid-template-columns: 1fr; gap: 0.875rem;">
            <div>
              <label class="nb-label" for="field-faq-question">Question</label>
              <input id="field-faq-question" class="nb-input" bind:value={faqForm.question} />
            </div>
            <div>
              <div class="nb-label">Answer</div>
              <MarkdownEditor bind:value={faqForm.answer} />
            </div>
            <div style="display: flex; align-items: center; gap: 0.5rem;">
              <input type="checkbox" id="field-common-question" bind:checked={faqForm.is_common_question} />
              <label class="nb-label" for="field-common-question" style="margin-bottom: 0;">Common Question</label>
            </div>
          </div>
        </div>
        <div style="display: flex; gap: 0.5rem;">
          <button style="background: transparent; border: 1px solid var(--nb-border); color: var(--nb-text2); border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; cursor: pointer;" onclick={cancelFaq}>Cancel</button>
          <button style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;" onclick={handleFaqSave} disabled={faqSaving}>{faqSaving ? 'Saving…' : 'Save'}</button>
        </div>
      {/if}

    {:else if activeTab === 'instructions'}
      {#if instrMode === 'list'}
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
          <span style="font-size: 0.875rem; font-weight: 500; color: var(--nb-text2);">AI Instructions</span>
          <button
            style="background: transparent; border: 1px solid var(--nb-border); border-radius: 0.25rem; padding: 0.5rem 1rem; color: var(--nb-text2); font-size: 0.8125rem; cursor: pointer;"
            onclick={startInstrCreate}
          >+ Add New</button>
        </div>
        {#each instrItems as item}
          <div style={cardStyle}>
            <div style={cardBodyStyle}>
              <div style={cardTitleStyle}>{item.instruction_type}</div>
              <div style={cardMetaStyle}>{item.instruction.length > 80 ? item.instruction.slice(0, 80) + '…' : item.instruction}</div>
              <span style={tagPillStyle}>priority: {item.priority}</span>
            </div>
            <div style={cardActionsStyle}>
              <button style={actionBtnStyle} onclick={() => startInstrEdit(item)}>Edit</button>
              <button style="{actionBtnStyle} color: var(--nb-red-text);" onclick={() => handleInstrDelete(item.id)}>Delete</button>
            </div>
          </div>
        {/each}
      {:else}
        <div style="background: var(--nb-bg2); border: 1px solid var(--nb-border); border-radius: 0.625rem; padding: 1.25rem; margin-bottom: 1rem;">
          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
            <div>
              <label class="nb-label" for="field-instr-type">Instruction Type</label>
              <input id="field-instr-type" class="nb-input" bind:value={instrForm.instruction_type} />
            </div>
            <div>
              <label class="nb-label" for="field-instr-priority">Priority</label>
              <input id="field-instr-priority" type="number" class="nb-input" bind:value={instrForm.priority} />
            </div>
            <div style="grid-column: 1 / -1;">
              <label class="nb-label" for="field-instr-body">Instruction</label>
              <textarea id="field-instr-body" class="nb-input" rows="5" bind:value={instrForm.instruction}></textarea>
            </div>
          </div>
        </div>
        <div style="display: flex; gap: 0.5rem;">
          <button style="background: transparent; border: 1px solid var(--nb-border); color: var(--nb-text2); border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; cursor: pointer;" onclick={cancelInstr}>Cancel</button>
          <button style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;" onclick={handleInstrSave} disabled={instrSaving}>{instrSaving ? 'Saving…' : 'Save'}</button>
        </div>
      {/if}
    {/if}
  {/if}
</div>
