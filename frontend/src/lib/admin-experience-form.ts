import type { ExperienceInput } from './admin-types';

/**
 * Default (new-entry) state for the experience admin form.
 *
 * Extracted from `+page.svelte` so it is importable and directly testable —
 * `emptyForm()` previously lived as a local function inside the route's
 * script block. See R-0006.2 (defaults `visible` to `true`) and the vitest
 * coverage in `__tests__/admin-experience-form.test.ts`.
 */
export function emptyExperienceForm(): ExperienceInput {
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
    visible: true,
  };
}
