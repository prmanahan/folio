/**
 * Derives monogram initials from a full name string.
 *
 * Rules:
 * - Two-or-more words: first letter of first word + first letter of last word, uppercased.
 *   "Peter Manahan" → "PM", "Alex Rivera" → "AR"
 * - Single word: first letter repeated twice, uppercased.
 *   "Madonna" → "MM"
 * - Empty string or whitespace-only: returns "?" as a static fallback.
 */
export function getInitials(name: string): string {
	const words = name.trim().split(/\s+/).filter(Boolean);
	if (words.length === 0) return '?';
	const first = words[0].charAt(0).toUpperCase();
	const last = words[words.length - 1].charAt(0).toUpperCase();
	return first + last;
}
