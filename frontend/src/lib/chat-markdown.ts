/**
 * Renders untrusted model output (chat answers) to safe HTML.
 *
 * Contract: `renderChatMarkdown(text)` returns an HTML string containing only a
 * markdown subset (p, br, hr, lists, tables, strong/em/del, code/pre,
 * blockquote, headings, links). No images, styles or form controls; links keep
 * `href` only for http(s)/mailto and always carry rel="noopener noreferrer".
 * Invisible characters used for hidden-instruction smuggling are stripped.
 *
 * Model output is attacker-steerable (anyone typing into the chat prompt), so
 * this is an allowlist, not DOMPurify's default profile.
 */
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import { stripInvisible } from '$lib/invisible';

const ALLOWED_TAGS = [
	'p', 'br', 'hr', 'ul', 'ol', 'li', 'strong', 'em', 'del', 'code', 'pre', 'blockquote', 'a',
	'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
	'table', 'thead', 'tbody', 'tr', 'th', 'td'
];

const FORBID_TAGS = [
	'img', 'picture', 'source', 'style', 'link', 'form', 'input', 'button', 'select', 'option',
	'textarea', 'label', 'fieldset', 'iframe', 'object', 'embed', 'svg', 'math'
];

const SAFE_HREF = /^(https?|mailto):/i;
const ALIGN_VALUES = new Set(['left', 'center', 'right']);

function restrictNode(node: Element): void {
	// `align` is cosmetic column alignment that marked emits on th/td only.
	if (node.hasAttribute('align')) {
		const tag = node.tagName;
		const ok = (tag === 'TH' || tag === 'TD') && ALIGN_VALUES.has(node.getAttribute('align')!);
		if (!ok) node.removeAttribute('align');
	}
	if (node.tagName !== 'A') return;
	const href = node.getAttribute('href');
	// Strip whitespace/control characters (code points <= 0x20) that browsers
	// ignore inside a scheme ("java<TAB>script:") before checking, so the
	// allowlist sees what the browser sees.
	const compact = href === null ? '' : [...href].filter((c) => c.codePointAt(0)! > 0x20).join('');
	if (!SAFE_HREF.test(compact)) node.removeAttribute('href');
	node.setAttribute('rel', 'noopener noreferrer');
}

export function sanitizeChatHtml(html: string): string {
	// The hook is scoped to this call: DOMPurify is a shared instance and the
	// article/project/admin renderers must not inherit the chat restrictions.
	DOMPurify.addHook('afterSanitizeAttributes', restrictNode);
	try {
		return DOMPurify.sanitize(html, {
			ALLOWED_TAGS,
			ALLOWED_ATTR: ['href', 'start', 'align'],
			FORBID_TAGS,
			ALLOW_DATA_ATTR: false,
			ALLOW_ARIA_ATTR: false
		});
	} finally {
		DOMPurify.removeHook('afterSanitizeAttributes', restrictNode);
	}
}

/**
 * Strips twice. marked and DOMPurify decode character references
 * (`&#x200B;`, `&#917569;`, `&ZeroWidthSpace;`) into real characters AFTER the
 * first strip, so the serialized output is stripped again. DOMPurify
 * serializes decoded characters literally, so one pass over the output
 * covers text nodes and attribute values alike.
 */
export function renderChatMarkdown(text: string): string {
	return stripInvisible(
		sanitizeChatHtml(marked.parse(stripInvisible(text), { async: false }))
	);
}
