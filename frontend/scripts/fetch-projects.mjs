#!/usr/bin/env node
// Fetches the public repo lists of the two owners from the GitHub API and
// writes frontend/static/projects.json (folio #4726; spec
// docs/specs/2026-11-11-projects-from-github.md R-1 to R-6, R-8).
//
// A fail-closed control over untrusted input: GitHub's repo text flows on to a
// public page and into an AI prompt. Anything this script cannot positively
// validate stops the run, and a stopped run leaves NO file at the output path
// (R-2). No retry, no fallback list, no partial output.
//
// Plain Node, built-in fetch, no dependency. Reads the token from the
// GITHUB_TOKEN environment variable only; the token is never logged, never
// written, and never placed in argv. Log lines carry owner names, validated
// repo names, status codes and code point numbers, never raw GitHub text.
//
// Usage: GITHUB_TOKEN=... node frontend/scripts/fetch-projects.mjs
//        (or `just fetch-projects`)

import { mkdirSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const OUT_PATH = join(dirname(fileURLToPath(import.meta.url)), '..', 'static', 'projects.json');
// The write goes through a sibling temp file and a rename, so no partial file is ever visible.
const TMP_PATH = `${OUT_PATH}.${process.pid}.tmp`;

// Loaded inside run(), after the stale file is deleted, so a broken table cannot leave one behind.
// null until then, so a use before the import throws instead of silently matching nothing.
let invisibleRanges = null;

// Group order is the output order (R-8).
const OWNERS = [
	{ owner: 'mnemra', url: 'https://api.github.com/orgs/mnemra/repos', query: { type: 'public' } },
	{ owner: 'prmanahan', url: 'https://api.github.com/users/prmanahan/repos', query: {} },
];
const MAX_PAGES = 10;
const REQUEST_TIMEOUT_MS = 30_000;
const PER_PAGE = 100;
const MAX_DESCRIPTION = 350;
const NAME_RE = /^[A-Za-z0-9._-]{1,100}$/;

/** A condition that stops the run. The message is safe to log. */
class RunError extends Error {}

const info = (line) => console.log(line);

/**
 * A closed token naming why a call failed: the error code (EACCES, UND_ERR_*), else the cause's
 * code, else the error name. err.message is never used: the runtime's text can carry a path, and
 * undici's invalid-header error can echo the token. Anything off the token shape becomes 'error'.
 */
function causeToken(err) {
	for (const c of [err?.code, err?.cause?.code, err?.name]) {
		if (typeof c === 'string' && /^[A-Za-z0-9_]{1,64}$/.test(c)) return c;
	}
	return 'error';
}

/** First rejected-class code point of `text`, or null. */
function rejectedCodePoint(text) {
	for (const ch of text) {
		const cp = ch.codePointAt(0);
		// for...of yields a lone surrogate as a single code point; the Rust reader rejects it.
		if (cp >= 0xd800 && cp <= 0xdfff) return cp;
		if (/\p{Cc}/u.test(ch) || cp === 0x2028 || cp === 0x2029) return cp;
		if (invisibleRanges.some(([lo, hi]) => cp >= lo && cp <= hi)) return cp;
	}
	return null;
}

const isPlainObject = (v) =>
	v !== null &&
	typeof v === 'object' &&
	!Array.isArray(v) &&
	(Object.getPrototypeOf(v) === Object.prototype || Object.getPrototypeOf(v) === null);

/** R-4: throws RunError naming the owner, the element index and the clause. */
function validateElement(el, owner, index) {
	const fail = (why) => {
		throw new RunError(`${owner}: element ${index} rejected: ${why}`);
	};
	if (!isPlainObject(el)) fail('not a plain object');
	if (el.private !== false) fail('private is not false');
	if (typeof el.fork !== 'boolean') fail('fork is not a boolean');
	if (typeof el.archived !== 'boolean') fail('archived is not a boolean');
	if (!isPlainObject(el.owner) || typeof el.owner.login !== 'string') fail('owner login missing or not a string');
	if (el.owner.login.toLowerCase() !== owner) fail('owner login differs from the owner requested');
	if (typeof el.name !== 'string' || !NAME_RE.test(el.name)) fail('name does not match the allowed pattern');
	const description = el.description;
	if (description !== null && typeof description !== 'string') fail('description is neither null nor a string');
	if (typeof description === 'string') {
		if ([...description].length > MAX_DESCRIPTION) fail(`description exceeds ${MAX_DESCRIPTION} code points`);
		const bad = rejectedCodePoint(description);
		if (bad !== null) fail(`description contains a rejected code point (U+${bad.toString(16).toUpperCase().padStart(4, '0')})`);
	}
}

/**
 * The URL of the single `rel="next"` link, resolved against `base`, or null.
 * More than one next link, or an unparseable one, fails the run.
 */
function nextLink(header, base, owner) {
	if (header === null || header === '') return null;
	const nexts = [];
	for (const m of header.matchAll(/<([^>]*)>([^<]*)/g)) {
		const rel = /;\s*rel\s*=\s*"?([^";,]*)"?/i.exec(m[2]);
		if (rel && rel[1].toLowerCase().split(/\s+/).includes('next')) nexts.push(m[1]);
	}
	if (nexts.length === 0) return null;
	if (nexts.length > 1) throw new RunError(`${owner}: more than one next link`);
	try {
		return new URL(nexts[0], base);
	} catch {
		throw new RunError(`${owner}: next link is not a URL`);
	}
}

/** R-1: one owner's pages, same-origin same-path links only, at most MAX_PAGES. */
async function fetchOwner({ owner, url, query }, token) {
	const first = new URL(url);
	for (const [k, v] of Object.entries(query)) first.searchParams.set(k, v);
	first.searchParams.set('per_page', String(PER_PAGE));
	const want = new URL(url);

	const elements = [];
	let next = first;
	for (let pageNo = 1; next !== null; pageNo++) {
		if (pageNo > MAX_PAGES) throw new RunError(`${owner}: more than ${MAX_PAGES} pages`);
		if (next.origin !== want.origin || next.pathname !== want.pathname || next.username !== '' || next.password !== '') {
			throw new RunError(`${owner}: next link leaves ${want.origin}${want.pathname}`);
		}
		let res;
		try {
			res = await fetch(next, {
				redirect: 'error',
				signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
				headers: {
					authorization: `Bearer ${token}`,
					accept: 'application/vnd.github+json',
					'x-github-api-version': '2022-11-28',
					'user-agent': 'folio-fetch-projects',
				},
			});
		} catch (err) {
			// err.message is not logged (see causeToken). A timeout and a redirect also land here.
			throw new RunError(`${owner}: request failed on page ${pageNo} (${causeToken(err)})`);
		}
		if (!res.ok) throw new RunError(`${owner}: page ${pageNo} returned HTTP ${res.status}`);
		let text;
		try {
			text = await res.text();
		} catch (err) {
			throw new RunError(`${owner}: page ${pageNo} body could not be read (${causeToken(err)})`);
		}
		let body;
		try {
			body = JSON.parse(text);
		} catch {
			throw new RunError(`${owner}: page ${pageNo} body is not JSON`);
		}
		if (!Array.isArray(body)) throw new RunError(`${owner}: page ${pageNo} body is not a JSON array`);
		elements.push(...body);
		next = nextLink(res.headers.get('link'), next, owner);
	}
	return elements;
}

/** ASCII-lowercase, compared by code unit; ties (impossible on GitHub) fall to the raw name. */
const asciiLower = (s) => s.replace(/[A-Z]/g, (c) => c.toLowerCase());
const byName = (a, b) => {
	const x = asciiLower(a.name);
	const y = asciiLower(b.name);
	if (x !== y) return x < y ? -1 : 1;
	return a.name < b.name ? -1 : a.name > b.name ? 1 : 0;
};

/** R-4 over every element, then R-3, then R-8 order. Logs per R-6. */
function buildGroup(owner, elements) {
	elements.forEach((el, i) => validateElement(el, owner, i));
	const included = [];
	for (const el of elements) {
		const reason = el.fork ? 'fork' : el.archived ? 'archived' : el.name.toLowerCase() === '.github' ? '.github' : null;
		if (reason !== null) info(`${owner}: excluded ${el.name} (${reason})`);
		else included.push({ name: el.name, description: el.description ?? '' });
	}
	if (included.length === 0) throw new RunError(`${owner}: zero repos after exclusions`);
	included.sort(byName);
	// Strictly increasing by ASCII-lowercased name, as the Rust validator requires: this also
	// rejects a repeated name and a case-only twin.
	for (let i = 1; i < included.length; i++) {
		if (asciiLower(included[i - 1].name) === asciiLower(included[i].name)) {
			throw new RunError(`${owner}: duplicate repo name ${included[i].name} (compared ASCII-case-insensitively) at sorted position ${i}`);
		}
	}
	info(`${owner}: included ${included.length}: ${included.map((r) => r.name).join(', ')}`);
	for (const r of included) if (r.description === '') info(`${owner}: ${r.name} has an empty description`);
	return { owner, repos: included };
}

async function run() {
	// R-2: the output goes first, so no failure below can leave a stale file.
	rmSync(OUT_PATH, { force: true });

	// D12: no token, no request.
	const token = process.env.GITHUB_TOKEN;
	if (token === undefined || token.trim() === '') {
		throw new RunError('GITHUB_TOKEN is not set or is empty; no request sent');
	}

	({ INVISIBLE_RANGES: invisibleRanges } = await import('./invisible-table.mjs'));

	const groups = [];
	for (const o of OWNERS) groups.push(buildGroup(o.owner, await fetchOwner(o, token)));

	const doc = { fetched_at: new Date().toISOString(), groups };
	mkdirSync(dirname(OUT_PATH), { recursive: true });
	writeFileSync(TMP_PATH, `${JSON.stringify(doc, null, '\t')}\n`);
	renameSync(TMP_PATH, OUT_PATH);
	info(`wrote ${OUT_PATH} (fetched_at ${doc.fetched_at})`);
}

try {
	await run();
} catch (err) {
	// Whatever went wrong, no file may remain. A failure to delete is itself a failure.
	// Each path in its own try, so a failed removal of one never skips the other.
	let cleanup = '';
	try {
		rmSync(TMP_PATH, { force: true });
	} catch {
		cleanup += ' (and the temp file could not be removed)';
	}
	try {
		rmSync(OUT_PATH, { force: true });
	} catch {
		cleanup += ' (and projects.json could not be removed)';
	}
	console.error(`fetch-projects failed: ${err instanceof RunError ? err.message : `unexpected ${causeToken(err)}`}${cleanup}`);
	process.exit(1);
}
