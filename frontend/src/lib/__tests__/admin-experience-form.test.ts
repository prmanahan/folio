import { describe, it, expect } from 'vitest';
import { emptyExperienceForm } from '$lib/admin-experience-form';

describe('emptyExperienceForm', () => {
	it('defaults visible to true (R-0006.2 / S6)', () => {
		expect(emptyExperienceForm().visible).toBe(true);
	});

	it('defaults is_current to false', () => {
		expect(emptyExperienceForm().is_current).toBe(false);
	});

	it('defaults text fields to empty strings', () => {
		const form = emptyExperienceForm();
		expect(form.company_name).toBe('');
		expect(form.title).toBe('');
	});
});
