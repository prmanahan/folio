<script lang="ts">
  import { onMount } from 'svelte';
  import { getProfile, updateProfile } from '$lib/admin-api';
  import type { ProfileInput } from '$lib/admin-types';
  import FormSection from '$lib/components/admin/FormSection.svelte';
  import TagInput from '$lib/components/admin/TagInput.svelte';
  import Toast from '$lib/components/admin/Toast.svelte';

  // Length limits mirror libs/site-core/models/profile.rs.
  // Source of truth is the Rust validator; these are admin-UI hints.
  const LIMITS = {
    name: 32,
    title: 48,
    pitch_short: 280,
    pitch_long: 1500,
    location: 48,
    remote_preference: 64,
    availability_status: 32,
  } as const;

  let form = $state<ProfileInput>({
    name: '',
    email: '',
    title: '',
    location: '',
    phone: '',
    linkedin_url: '',
    github_url: '',
    twitter_url: '',
    pitch_short: '',
    pitch_long: '',
    availability_status: '',
    availability_date: '',
    remote_preference: '',
    target_titles: [],
    target_company_stages: [],
    career_narrative: '',
    looking_for: '',
    not_looking_for: '',
    management_style: '',
    work_style: '',
    salary_min: null,
    salary_max: null,
  });

  // Count Unicode scalars, matching the Rust validator's `chars().count()`.
  function charCount(s: string): number {
    return Array.from(s).length;
  }

  let pitchShortCount = $derived(charCount(form.pitch_short));
  let pitchLongCount = $derived(charCount(form.pitch_long));
  let pitchShortOver = $derived(pitchShortCount > LIMITS.pitch_short);
  let pitchShortEmpty = $derived(form.pitch_short.trim().length === 0);
  let pitchLongOver = $derived(pitchLongCount > LIMITS.pitch_long);
  let nameOver = $derived(charCount(form.name) > LIMITS.name);
  let titleOver = $derived(charCount(form.title) > LIMITS.title);
  let locationOver = $derived(charCount(form.location) > LIMITS.location);
  let remotePrefOver = $derived(charCount(form.remote_preference) > LIMITS.remote_preference);
  let availStatusOver = $derived(charCount(form.availability_status) > LIMITS.availability_status);

  let validationBlocked = $derived(
    pitchShortOver ||
      pitchShortEmpty ||
      pitchLongOver ||
      nameOver ||
      titleOver ||
      locationOver ||
      remotePrefOver ||
      availStatusOver,
  );
  let loading = $state(true);
  let saving = $state(false);
  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');
  let isDirty = $state(false);

  onMount(async () => {
    try {
      const profile = await getProfile();
      form = {
        name: profile.name,
        email: profile.email,
        title: profile.title,
        location: profile.location,
        phone: profile.phone,
        linkedin_url: profile.linkedin_url,
        github_url: profile.github_url,
        twitter_url: profile.twitter_url,
        pitch_short: profile.pitch_short,
        pitch_long: profile.pitch_long,
        availability_status: profile.availability_status,
        availability_date: profile.availability_date,
        remote_preference: profile.remote_preference,
        target_titles: profile.target_titles,
        target_company_stages: profile.target_company_stages,
        career_narrative: profile.career_narrative,
        looking_for: profile.looking_for,
        not_looking_for: profile.not_looking_for,
        management_style: profile.management_style,
        work_style: profile.work_style,
        salary_min: profile.salary_min,
        salary_max: profile.salary_max,
      };
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to load profile';
      toastType = 'error';
    } finally {
      loading = false;
    }
  });

  async function handleSave() {
    if (validationBlocked) {
      toastMessage = 'Fix the highlighted fields before saving';
      toastType = 'error';
      return;
    }
    saving = true;
    try {
      await updateProfile(form);
      isDirty = false;
      toastMessage = 'Profile saved';
      toastType = 'success';
    } catch (err) {
      toastMessage = err instanceof Error ? err.message : 'Failed to save';
      toastType = 'error';
    } finally {
      saving = false;
    }
  }
</script>

<Toast bind:message={toastMessage} bind:type={toastType} />

<div style="padding: 1.5rem 2rem 3rem; max-width: 56.25rem;">
  <div style="display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 1.5rem;">
    <h1 style="font-size: 1.125rem; font-weight: 500; color: var(--nb-text); letter-spacing: -0.01em;">Profile</h1>
    <button
      style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;"
      onclick={handleSave}
      disabled={saving || validationBlocked}
      data-testid="save-profile-btn"
    >
      {saving ? 'Saving…' : 'Save Profile'}
    </button>
  </div>

  {#if loading}
    <div style="color: var(--nb-text3); font-size: 0.875rem; padding: 1rem 0;">Loading…</div>
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div oninput={() => isDirty = true}>
      <FormSection title="Public Information" tier="public">
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
          <div>
            <label class="nb-label" for="field-name">Name</label>
            <input id="field-name" class="nb-input" maxlength={LIMITS.name} bind:value={form.name} />
            {#if nameOver}
              <div class="field-error" data-testid="error-name">Max {LIMITS.name} chars</div>
            {/if}
          </div>
          <div>
            <label class="nb-label" for="field-email">Email</label>
            <input id="field-email" class="nb-input" type="email" bind:value={form.email} />
          </div>
          <div>
            <label class="nb-label" for="field-title">Title</label>
            <input id="field-title" class="nb-input" maxlength={LIMITS.title} bind:value={form.title} />
            {#if titleOver}
              <div class="field-error" data-testid="error-title">Max {LIMITS.title} chars</div>
            {/if}
          </div>
          <div>
            <label class="nb-label" for="field-location">Location</label>
            <input id="field-location" class="nb-input" maxlength={LIMITS.location} bind:value={form.location} />
            {#if locationOver}
              <div class="field-error" data-testid="error-location">Max {LIMITS.location} chars</div>
            {/if}
          </div>
          <div>
            <label class="nb-label" for="field-phone">Phone</label>
            <input id="field-phone" class="nb-input" bind:value={form.phone} />
          </div>
          <div>
            <label class="nb-label" for="field-linkedin">LinkedIn URL</label>
            <input id="field-linkedin" class="nb-input" bind:value={form.linkedin_url} />
          </div>
          <div>
            <label class="nb-label" for="field-github">GitHub URL</label>
            <input id="field-github" class="nb-input" bind:value={form.github_url} />
          </div>
          <div>
            <label class="nb-label" for="field-twitter">Twitter URL</label>
            <input id="field-twitter" class="nb-input" bind:value={form.twitter_url} />
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-pitch-short">
              Pitch (short — hub) — required
            </label>
            <textarea
              id="field-pitch-short"
              class="nb-input"
              rows="3"
              maxlength={LIMITS.pitch_short}
              bind:value={form.pitch_short}
              data-testid="field-pitch-short"
            ></textarea>
            <div class="counter-row">
              <span
                class="char-counter"
                class:over={pitchShortOver}
                data-testid="counter-pitch-short"
              >{pitchShortCount} / {LIMITS.pitch_short}</span>
              {#if pitchShortEmpty}
                <span class="field-error" data-testid="error-pitch-short-empty">Required</span>
              {:else if pitchShortOver}
                <span class="field-error" data-testid="error-pitch-short-over">Over limit</span>
              {/if}
            </div>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-pitch-long">
              Pitch (long — resume / AI context)
            </label>
            <textarea
              id="field-pitch-long"
              class="nb-input"
              rows="6"
              maxlength={LIMITS.pitch_long}
              bind:value={form.pitch_long}
              data-testid="field-pitch-long"
            ></textarea>
            <div class="counter-row">
              <span
                class="char-counter"
                class:over={pitchLongOver}
                data-testid="counter-pitch-long"
              >{pitchLongCount} / {LIMITS.pitch_long}</span>
              {#if pitchLongOver}
                <span class="field-error" data-testid="error-pitch-long-over">Over limit</span>
              {/if}
            </div>
          </div>
          <div>
            <label class="nb-label" for="field-availability-status">Availability Status</label>
            <input
              id="field-availability-status"
              class="nb-input"
              maxlength={LIMITS.availability_status}
              bind:value={form.availability_status}
            />
            {#if availStatusOver}
              <div class="field-error" data-testid="error-availability-status">
                Max {LIMITS.availability_status} chars
              </div>
            {/if}
          </div>
          <div>
            <label class="nb-label" for="field-availability-date">Availability Date</label>
            <input id="field-availability-date" class="nb-input" bind:value={form.availability_date} />
          </div>
          <div>
            <label class="nb-label" for="field-remote-preference">Remote Preference</label>
            <input
              id="field-remote-preference"
              class="nb-input"
              maxlength={LIMITS.remote_preference}
              bind:value={form.remote_preference}
            />
            {#if remotePrefOver}
              <div class="field-error" data-testid="error-remote-preference">
                Max {LIMITS.remote_preference} chars
              </div>
            {/if}
          </div>
        </div>
      </FormSection>

      <FormSection title="AI Context" tier="ai">
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-target-titles">Target Titles</label>
            <TagInput bind:value={form.target_titles} placeholder="Add title" />
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-target-stages">Target Company Stages</label>
            <TagInput bind:value={form.target_company_stages} placeholder="Add stage" />
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-career-narrative">Career Narrative</label>
            <textarea id="field-career-narrative" class="nb-input" rows="4" bind:value={form.career_narrative}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-looking-for">Looking For</label>
            <textarea id="field-looking-for" class="nb-input" rows="3" bind:value={form.looking_for}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-not-looking-for">Not Looking For</label>
            <textarea id="field-not-looking-for" class="nb-input" rows="3" bind:value={form.not_looking_for}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-management-style">Management Style</label>
            <textarea id="field-management-style" class="nb-input" rows="3" bind:value={form.management_style}></textarea>
          </div>
          <div style="grid-column: 1 / -1;">
            <label class="nb-label" for="field-work-style">Work Style</label>
            <textarea id="field-work-style" class="nb-input" rows="3" bind:value={form.work_style}></textarea>
          </div>
        </div>
      </FormSection>

      <FormSection title="Private Information" tier="private" collapsed>
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem; margin-top: 0.75rem;">
          <div>
            <label class="nb-label" for="field-salary-min">Salary Minimum</label>
            <input id="field-salary-min" type="number" class="nb-input" bind:value={form.salary_min} />
          </div>
          <div>
            <label class="nb-label" for="field-salary-max">Salary Maximum</label>
            <input id="field-salary-max" type="number" class="nb-input" bind:value={form.salary_max} />
          </div>
        </div>
      </FormSection>
    </div>

    <div style="margin-top: 1rem; display: flex; align-items: center; gap: 0.75rem;">
      <button
        style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;"
        onclick={handleSave}
        disabled={saving}
      >
        {saving ? 'Saving…' : 'Save Profile'}
      </button>
      {#if isDirty}
        <span style="display: flex; align-items: center; gap: 0.375rem;">
          <span style="width: 6px; height: 6px; border-radius: 50%; background: var(--nb-amber);"></span>
          <span style="font-size: 0.6875rem; color: var(--nb-text3);">Unsaved changes</span>
        </span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .counter-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 0.25rem;
    font-size: 0.6875rem;
  }
  .char-counter {
    color: var(--nb-text3);
    font-variant-numeric: tabular-nums;
  }
  .char-counter.over {
    color: var(--nb-amber, #d97706);
    font-weight: 600;
  }
  .field-error {
    color: var(--nb-amber, #d97706);
    font-size: 0.6875rem;
    margin-top: 0.25rem;
  }
</style>
