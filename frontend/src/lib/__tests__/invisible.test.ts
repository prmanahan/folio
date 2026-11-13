import { describe, it, expect } from 'vitest';
import { stripInvisible } from '#lib/invisible.ts';
import { INVISIBLE_RANGES } from '#lib/invisible-table.ts';

// Characters are built from code points, never written as literals.
const cp = (...n: number[]) => String.fromCodePoint(...n);

describe('stripInvisible', () => {
	// Membership oracle built by plain iteration over the table, independent of
	// the binary search in the implementation.
	const members = new Set<number>();
	for (const [lo, hi] of INVISIBLE_RANGES) for (let c = lo; c <= hi; c++) members.add(c);

	it('positive control: the oracle holds known Default_Ignorable members', () => {
		for (const known of [
			0x00ad, 0x034f, 0x061c, 0x115f, 0x180b, 0x200b, 0x200d, 0x200f, 0x202e, 0x2060,
			0x2064, 0x206f, 0x3164, 0xfe00, 0xfe0e, 0xfe0f, 0xfeff, 0xffa0, 0xfff9, 0xfffb,
			0x1d173, 0xe0001, 0xe0041, 0xe007f, 0xe0100, 0xe01ef
		]) {
			expect(members.has(known), `U+${known.toString(16)}`).toBe(true);
		}
		for (const visible of [0x41, 0x20, 0x2764, 0x200a, 0x2070, 0x1f600]) {
			expect(members.has(visible), `U+${visible.toString(16)}`).toBe(false);
		}
	});

	it('strips a code point exactly when it is in the table (every scalar value)', () => {
		const wrong: string[] = [];
		for (let c = 0; c <= 0x10ffff; c++) {
			if (c >= 0xd800 && c <= 0xdfff) continue; // not scalar values
			const out = stripInvisible(`a${cp(c)}b`);
			const expected = members.has(c) ? 'ab' : `a${cp(c)}b`;
			if (out !== expected) wrong.push(`U+${c.toString(16)}`);
		}
		expect(wrong).toEqual([]);
	});

	it('strips variation selectors with no exceptions, including after a visible character', () => {
		expect(stripInvisible(cp(0x2764, 0xfe0f))).toBe(cp(0x2764));
		expect(stripInvisible(cp(0x2764, 0xfe0e))).toBe(cp(0x2764));
		expect(stripInvisible(`x${cp(0xfe0f, 0xfe0f)}y`)).toBe('xy');
	});

	it('leaves ordinary text, including astral characters, alone', () => {
		const s = `plain text ${cp(0x1f600)} 100% done`;
		expect(stripInvisible(s)).toBe(s);
	});
});
