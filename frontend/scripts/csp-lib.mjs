// The CSP guard's logic (Warden, #4710), as a library so tests import it and
// the CLI (check-csp.mjs) calls it unconditionally. See check-csp.mjs for usage.
//
// Parsed with a real HTML parser (jsdom) so comments, <noscript>, attribute
// quoting and element order are what a browser sees. Fails closed: unreadable
// or unparseable input is a failure.
//
// Rules, each one a bypass an earlier version let through:
//  - exactly one enforce-mode CSP meta, in <head>, before any <script>: a meta
//    only applies to what follows it. `http-equiv` is compared RAW and
//    ASCII-lowercased, exactly as browsers do: a leading or trailing space,
//    tab or NBSP makes browsers ignore the tag, so it must not count here;
//  - no `on*` attribute on any element that precedes the meta (Firefox runs
//    those before the policy applies);
//  - the policy is read with a port of the W3C "parse a serialized CSP"
//    algorithm (ASCII whitespace only; first duplicate wins), and any token it
//    would skip (non-ASCII) or ignore (duplicate directive) FAILS the guard;
//  - the raw meta content must match the SHAPE our build emits (shapeErrors):
//    directives joined by exactly "; ", tokens by exactly one U+0020, names from
//    the set configured in svelte.config.js kit.csp.directives, value tokens
//    printable ASCII without "," or ";". This is an allowlist, so input the
//    spec parser reads one way and an engine another (VT, C0, comma) fails
//    without this file predicting what any engine does;
//  - no <noscript> before the meta: jsdom parses with scripting off, a browser
//    with it on, and a <noscript> in <head> parses differently between the two,
//    so markup there is not reliably what this guard saw. The build emits none;
//  - script-src, script-src-elem and script-src-attr are allowlisted to
//    'self', 'sha256-...' and 'none'; anything else fails (including
//    'unsafe-inline', 'unsafe-eval', 'unsafe-hashes', hosts, schemes, '*',
//    nonces, 'strict-dynamic');
//  - script-src-attr must be absent or exactly 'none';
//  - with no script-src, default-src is the effective script policy and is
//    held to the same allowlist;
//  - the meta policy must not carry frame-ancestors: a meta cannot enforce it,
//    and the server adds it to the HTTP header (see headerValue);
//  - ENCODING (checkCspBytes): the file is read as bytes. The only BOM allowed
//    is EF BB BF; the bytes must be valid UTF-8 (fatal TextDecoder); and no
//    <meta charset> / <meta http-equiv="content-type"> that jsdom's tree
//    exposes may name another charset. Declarations inside <template> or
//    <title> are not seen. The boundary is the server's charset=utf-8
//    Content-Type, accepted as docs/RISK-REGISTER.md RISK-1.
import { JSDOM } from 'jsdom';
import config from '../svelte.config.js';

// Directive names the build emits, read from the same config that emits them.
const ALLOWED_DIRECTIVES = new Set(Object.keys(config.kit.csp.directives));
// Value token: printable ASCII U+0021..U+007E, excluding "," and ";".
const VALUE_TOKEN = /^[\x21-\x2b\x2d-\x3a\x3c-\x7e]+$/;

const SCRIPT_DIRECTIVES = ['script-src', 'script-src-elem', 'script-src-attr'];
const ALLOWED_SOURCE = /^(?:'self'|'none'|'sha256-[A-Za-z0-9+/_-]+={0,2}')$/;

/** ASCII-only lowercase, as the HTML spec's "ASCII case-insensitive" match. */
const asciiLower = (s) => s.replace(/[A-Z]/g, (c) => c.toLowerCase());

/**
 * ASCII whitespace: U+0009, U+000A, U+000C, U+000D, U+0020
 * (infra.spec.whatwg.org/#ascii-whitespace). Deliberately NOT JS `\s` / trim(),
 * which also match NBSP, U+FEFF, U+3000 and more that browsers do not.
 */
const ASCII_WS = '\\t\\n\\f\\r ';
const ASCII_WS_RUN = new RegExp(`[${ASCII_WS}]+`);
const ASCII_WS_START = new RegExp(`^[${ASCII_WS}]+`);
const ASCII_WS_END = new RegExp(`[${ASCII_WS}]+$`);
// An ASCII string has only code points U+0000..U+007F (infra.spec.whatwg.org/#ascii-string).
const ASCII_STRING = /^[\x00-\x7f]*$/;

/**
 * "Parse a serialized CSP", w3c.github.io/webappsec-csp/#parse-serialized-policy,
 * ported step for step so the guard reads a policy as browsers do.
 *
 * A <meta http-equiv> policy goes through THIS algorithm directly, not through
 * "parse a serialized CSP list" (the comma-splitting one, used for the
 * Content-Security-Policy HTTP header): a comma in a meta policy is just a
 * character inside a token.
 *
 * Returns { directives, errors }. directives is the spec's directive set
 * (name lowercase -> value list). errors is the fail-closed addition: every
 * token the spec would SKIP (not ASCII) or IGNORE (duplicate name) is reported,
 * since our build emits neither, so one appearing is a defect or tampering.
 */
export function parsePolicy(serialized) {
	const directives = new Map(); // spec: policy's directive set
	const errors = [];
	// Spec: "For each token returned by splitting serialized on U+003B (;)".
	for (let token of serialized.split(';')) {
		// Spec step 1: strip leading and trailing ASCII whitespace from token.
		token = token.replace(ASCII_WS_START, '').replace(ASCII_WS_END, '');
		// Spec step 2: if token is an empty string, continue.
		if (token === '') continue;
		// Spec step 2: if token is not an ASCII string, continue (browsers skip it).
		// Fail closed: report it instead of silently dropping it.
		if (!ASCII_STRING.test(token)) {
			errors.push(`CSP token is not ASCII, so browsers skip it: ${JSON.stringify(token)}`);
			continue;
		}
		// Spec step 3 (name) and step 6 (value): split token on ASCII whitespace;
		// the first piece is the directive name, the rest is the value list.
		const [rawName, ...value] = token.split(ASCII_WS_RUN);
		// Spec step 4: directive name = ASCII lowercase of directive name.
		const name = asciiLower(rawName);
		// Spec step 5: if the directive set already has this name, continue (first wins).
		// Fail closed: report the ignored duplicate.
		if (directives.has(name)) {
			errors.push(`duplicate CSP directive ${name} (browsers ignore all but the first)`);
			continue;
		}
		// Spec steps 7-8: build the directive and append it to the directive set.
		directives.set(name, value);
	}
	return { directives, errors };
}

/**
 * Allowlist on the raw meta content (see header). Returns failure reasons.
 * Reports only the first violation: any one means the text is not our shape.
 */
export function shapeErrors(policy) {
	const show = JSON.stringify(policy);
	for (const directive of policy.split('; ')) {
		const tokens = directive.split(' ');
		if (tokens.includes('')) {
			return [`CSP policy shape: empty directive or non-single-space separator in ${show}`];
		}
		if (!ALLOWED_DIRECTIVES.has(tokens[0])) {
			return [`CSP policy shape: directive name not in svelte.config.js kit.csp.directives: ${JSON.stringify(tokens[0])}`];
		}
		for (const token of tokens.slice(1)) {
			if (!VALUE_TOKEN.test(token)) {
				return [`CSP policy shape: value token outside printable ASCII without "," or ";": ${JSON.stringify(token)}`];
			}
		}
	}
	return [];
}

const FRAME_ANCESTORS_SUFFIX = "; frame-ancestors 'none'";

/**
 * The exact Content-Security-Policy HTTP header value the server sends: the
 * validated meta policy plus frame-ancestors 'none' (meta cannot carry it).
 */
export function headerValue(metaPolicy) {
	return metaPolicy + FRAME_ANCESTORS_SUFFIX;
}

/** Decode a charset label as the guard requires: utf-8 only, ASCII case-insensitive. */
const isUtf8Label = (v) => asciiLower(v.replace(ASCII_WS_START, '').replace(ASCII_WS_END, '')) === 'utf-8';

/** Failure reasons for a declared charset other than utf-8. */
function charsetErrors(doc) {
	const errors = [];
	for (const m of doc.querySelectorAll('meta')) {
		if (m.hasAttribute('charset') && !isUtf8Label(m.getAttribute('charset'))) {
			errors.push(`document declares charset ${JSON.stringify(m.getAttribute('charset'))}, not utf-8`);
		}
		if (asciiLower(m.getAttribute('http-equiv') ?? '') === 'content-type') {
			const found = /charset[\t\n\f\r ]*=[\t\n\f\r ]*(?:"([^"]*)"|'([^']*)'|([^\t\n\f\r ;"']*))/i.exec(
				m.getAttribute('content') ?? ''
			);
			const label = found ? (found[1] ?? found[2] ?? found[3] ?? '') : null;
			if (label !== null && !isUtf8Label(label)) {
				errors.push(`document declares charset ${JSON.stringify(label)} in http-equiv content-type, not utf-8`);
			}
		}
	}
	return errors;
}

/**
 * Bytes -> { errors, header }. header is the exact header value when errors is
 * empty, otherwise null. Encoding rules first: anything but UTF-8 (with at most
 * an EF BB BF BOM) fails before the HTML is parsed.
 */
export function checkCspBytes(bytes) {
	const b = Buffer.from(bytes);
	const bom16 = b.length >= 2 && ((b[0] === 0xff && b[1] === 0xfe) || (b[0] === 0xfe && b[1] === 0xff));
	if (bom16) return { errors: ['non-UTF-8 byte order mark (UTF-16); only UTF-8 is allowed'], header: null };
	let html;
	try {
		html = new TextDecoder('utf-8', { fatal: true }).decode(b); // strips an EF BB BF BOM
	} catch {
		return { errors: ['file is not valid UTF-8'], header: null };
	}
	const errors = checkCsp(html);
	if (errors.length > 0) return { errors, header: null };
	// checkCsp passed, so exactly one CSP meta with a content attribute exists.
	const doc = new JSDOM(html).window.document;
	const meta = [...doc.querySelectorAll('meta')].find(
		(m) => asciiLower(m.getAttribute('http-equiv') ?? '') === 'content-security-policy'
	);
	return { errors: [], header: headerValue(meta.getAttribute('content')) };
}

/** Returns a list of failure reasons; empty means the page passes. */
export function checkCsp(html) {
	const errors = [];
	let doc;
	try {
		doc = new JSDOM(html).window.document;
	} catch (e) {
		return [`cannot parse HTML: ${e.message}`];
	}

	errors.push(...charsetErrors(doc));

	// No trim: browsers compare the raw attribute value.
	const metas = [...doc.querySelectorAll('meta')].filter(
		(m) => asciiLower(m.getAttribute('http-equiv') ?? '') === 'content-security-policy'
	);
	if (metas.length === 0) {
		return [...errors, 'no enforce-mode <meta http-equiv="content-security-policy"> tag'];
	}
	if (metas.length > 1) errors.push(`${metas.length} CSP meta tags (exactly one required)`);

	const meta = metas[0];
	if (meta.parentElement !== doc.head) errors.push('CSP meta is not a direct child of <head>');
	// Node.DOCUMENT_POSITION_PRECEDING (2): the other node comes before the meta.
	// Ancestors report PRECEDING too, so <html onload=...> is caught as well.
	for (const script of doc.querySelectorAll('script')) {
		if (meta.compareDocumentPosition(script) & 2) {
			errors.push('a <script> precedes the CSP meta tag');
			break;
		}
	}
	for (const el of doc.querySelectorAll('*')) {
		if (!(meta.compareDocumentPosition(el) & 2)) continue;
		const handler = [...el.attributes].find((a) => a.name.toLowerCase().startsWith('on'));
		if (handler) {
			errors.push(`<${el.localName}> before the CSP meta has an inline handler (${handler.name})`);
			break;
		}
	}

	for (const noscript of doc.querySelectorAll('noscript')) {
		if (meta.compareDocumentPosition(noscript) & 2) {
			errors.push('a <noscript> precedes the CSP meta tag');
			break;
		}
	}

	const policy = meta.getAttribute('content');
	if (policy === null) {
		errors.push('CSP meta has no content attribute');
		return errors;
	}
	errors.push(...shapeErrors(policy));
	const parsed = parsePolicy(policy);
	errors.push(...parsed.errors);
	const directives = parsed.directives;

	if (directives.has('frame-ancestors')) {
		errors.push('CSP meta policy contains frame-ancestors (meta cannot enforce it; the server header adds it)');
	}

	if (!directives.has('script-src') && !directives.has('default-src')) {
		errors.push('CSP has neither script-src nor default-src');
	}
	const checked = [...SCRIPT_DIRECTIVES];
	if (!directives.has('script-src')) checked.push('default-src');
	for (const name of checked) {
		for (const source of directives.get(name) ?? []) {
			if (!ALLOWED_SOURCE.test(source)) {
				errors.push(`${name} contains disallowed source ${source}`);
			}
		}
	}
	const attr = directives.get('script-src-attr');
	if (attr !== undefined && !(attr.length === 1 && attr[0] === "'none'")) {
		errors.push("script-src-attr must be absent or exactly 'none'");
	}
	return errors;
}
