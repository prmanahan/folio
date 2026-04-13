import { describe, it, expect } from 'vitest';
import { getInitials } from '$lib/utils';

describe('getInitials', () => {
	it('returns first + last initial for two-word name', () => {
		expect(getInitials('Peter Manahan')).toBe('PM');
	});

	it('returns first + last initial for two-word name (Alex Rivera)', () => {
		expect(getInitials('Alex Rivera')).toBe('AR');
	});

	it('returns first + last initial for three-word name', () => {
		expect(getInitials('Mary Jane Watson')).toBe('MW');
	});

	it('repeats first initial for single-word name', () => {
		expect(getInitials('Madonna')).toBe('MM');
	});

	it('uppercases initials', () => {
		expect(getInitials('alex rivera')).toBe('AR');
	});

	it('handles leading/trailing whitespace', () => {
		expect(getInitials('  Peter Manahan  ')).toBe('PM');
	});

	it('handles multiple spaces between words', () => {
		expect(getInitials('Peter  Manahan')).toBe('PM');
	});

	it('returns "?" for empty string', () => {
		expect(getInitials('')).toBe('?');
	});

	it('returns "?" for whitespace-only string', () => {
		expect(getInitials('   ')).toBe('?');
	});
});
