<script lang="ts">
  import { onMount } from 'svelte';
  import { getProfile, updateProfile } from '$lib/admin-api';
  import type { ProfileInput } from '$lib/admin-types';
  import FormSection from '$lib/components/admin/FormSection.svelte';
  import TagInput from '$lib/components/admin/TagInput.svelte';
  import Toast from '$lib/components/admin/Toast.svelte';

  let form = $state<ProfileInput>({
    name: '',
    email: '',
    title: '',
    location: '',
    phone: '',
    linkedin_url: '',
    github_url: '',
    twitter_url: '',
    elevator_pitch: '',
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
  let loading = $state(true);
  let saving = $state(false);
  let toastMessage = $state('');
  let toastType = $state<'success' | 'error'>('success');

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
        elevator_pitch: profile.elevator_pitch,
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
    saving = true;
    try {
      await updateProfile(form);
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
      disabled={saving}
    >
      {saving ? 'Saving…' : 'Save Profile'}
    </button>
  </div>

  {#if loading}
    <div style="color: var(--nb-text3); font-size: 0.875rem; padding: 1rem 0;">Loading…</div>
  {:else}
    <FormSection title="Public Information" tier="public">
      <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.875rem;">
        <div>
          <label class="nb-label" for="field-name">Name</label>
          <input id="field-name" class="nb-input" bind:value={form.name} />
        </div>
        <div>
          <label class="nb-label" for="field-email">Email</label>
          <input id="field-email" class="nb-input" type="email" bind:value={form.email} />
        </div>
        <div>
          <label class="nb-label" for="field-title">Title</label>
          <input id="field-title" class="nb-input" bind:value={form.title} />
        </div>
        <div>
          <label class="nb-label" for="field-location">Location</label>
          <input id="field-location" class="nb-input" bind:value={form.location} />
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
          <label class="nb-label" for="field-elevator-pitch">Elevator Pitch</label>
          <textarea id="field-elevator-pitch" class="nb-input" rows="4" bind:value={form.elevator_pitch}></textarea>
        </div>
        <div>
          <label class="nb-label" for="field-availability-status">Availability Status</label>
          <input id="field-availability-status" class="nb-input" bind:value={form.availability_status} />
        </div>
        <div>
          <label class="nb-label" for="field-availability-date">Availability Date</label>
          <input id="field-availability-date" class="nb-input" bind:value={form.availability_date} />
        </div>
        <div>
          <label class="nb-label" for="field-remote-preference">Remote Preference</label>
          <input id="field-remote-preference" class="nb-input" bind:value={form.remote_preference} />
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

    <div style="margin-top: 1rem;">
      <button
        style="background: var(--nb-gold); color: var(--nb-bg); border: none; border-radius: 0.25rem; padding: 0.5rem 1rem; font-size: 0.8125rem; font-weight: 600; cursor: pointer;"
        onclick={handleSave}
        disabled={saving}
      >
        {saving ? 'Saving…' : 'Save Profile'}
      </button>
    </div>
  {/if}
</div>
