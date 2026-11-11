import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import FitVerdictView from '$lib/components/ai/FitVerdict.svelte';
import type { FitVerdict } from '$lib/types';

function verdict(value: string): FitVerdict {
	return {
		verdict: value as FitVerdict['verdict'],
		headline: 'Headline',
		opening: 'Opening',
		gaps: [],
		transfers: [],
		recommendation: 'Talk to Peter'
	};
}

describe('FitVerdict invisible Unicode', () => {
	it('strips invisible characters from every model-written text field', () => {
		const z = String.fromCodePoint(0x200b);
		const tag = String.fromCodePoint(0xe0041);
		render(FitVerdictView, {
			props: {
				verdict: {
					verdict: 'strong_fit',
					headline: `Head${z}line`,
					opening: `Open${tag}ing`,
					gaps: [{ requirement: `Req${z}`, gap_title: `Gap${tag}`, explanation: `Expl${z}ain` }],
					transfers: [{ skill: `Sk${z}ill`, relevance: `Rel${tag}evance` }],
					recommendation: `Rec${z}ommend`
				}
			}
		});
		for (const text of ['Headline', 'Opening', 'Req', 'Gap', 'Explain', 'Skill', 'Relevance', 'Recommend']) {
			expect(screen.getByText(text)).toBeInTheDocument();
		}
	});
});

describe('FitVerdict badge', () => {
	it.each([
		['strong_fit', 'Strong Fit', 'verdict-strong'],
		['worth_conversation', 'Worth a Conversation', 'verdict-conversation'],
		['probably_not', 'Probably Not a Fit', 'verdict-not']
	])('renders %s as "%s"', (value, label, cssClass) => {
		render(FitVerdictView, { props: { verdict: verdict(value) } });
		const badge = screen.getByText(label);
		expect(badge).toHaveClass('verdict-badge', cssClass);
	});

	it.each(['something_new', '', 'constructor', '__proto__', 'toString'])(
		'renders a neutral "Unrated" badge instead of throwing for unknown verdict %j',
		(value) => {
			render(FitVerdictView, { props: { verdict: verdict(value) } });
			const badge = screen.getByText('Unrated');
			expect(badge).toHaveClass('verdict-badge', 'verdict-unknown');
			// The rest of the verdict still renders.
			expect(screen.getByText('Headline')).toBeInTheDocument();
			expect(screen.getByText('Talk to Peter')).toBeInTheDocument();
		}
	);
});
