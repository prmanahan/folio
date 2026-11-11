// Proves the CSP guard fails closed: every bad fixture must fail for its own
// reason, every good one must pass, and the CLI exits accordingly. Run with
// `npm run test:csp` (node:test, no vitest needed).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdtempSync, readdirSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { checkCsp, checkCspBytes, parsePolicy, shapeErrors } from './csp-lib.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const dir = join(here, 'csp-fixtures');
const cli = join(here, 'check-csp.mjs');
const files = readdirSync(dir).filter((f) => f.endsWith('.html')).sort();
const bad = files.filter((f) => f.startsWith('bad-'));
const good = files.filter((f) => f.startsWith('good'));

// Each bad fixture must fail for ITS reason, not an incidental one.
const REASON = {
	'bad-default-src-fallback.html': "default-src contains disallowed source 'unsafe-inline'",
	'bad-duplicate-first-unsafe.html': 'duplicate CSP directive script-src',
	'bad-duplicate-safe-first.html': 'duplicate CSP directive script-src',
	'bad-charset-http-equiv-latin1.html': 'in http-equiv content-type, not utf-8',
	'bad-charset-meta-iso-2022-jp.html': 'document declares charset "iso-2022-jp", not utf-8',
	'bad-charset-meta-uppercase-other.html': 'document declares charset "UTF-16LE", not utf-8',
	'bad-feff-before-name.html': 'CSP token is not ASCII',
	'bad-feff-between-name-value.html': 'CSP token is not ASCII',
	'bad-ideographic-space-before-name.html': 'CSP token is not ASCII',
	'bad-ideographic-space-between-name-value.html': 'CSP token is not ASCII',
	'bad-nbsp-before-name.html': 'CSP token is not ASCII',
	'bad-nbsp-between-name-value.html': 'CSP token is not ASCII',
	'bad-onattr-on-html-ancestor.html': 'has an inline handler (onfocus)',
	'bad-frame-ancestors-in-meta.html': 'CSP meta policy contains frame-ancestors',
	'bad-host-source.html': 'script-src contains disallowed source https://cdn.example.com',
	'bad-invalid-utf8.html': 'file is not valid UTF-8',
	'bad-http-equiv-leading-space.html': 'no enforce-mode',
	'bad-http-equiv-leading-tab.html': 'no enforce-mode',
	'bad-http-equiv-trailing-nbsp.html': 'no enforce-mode',
	'bad-http-equiv-trailing-space.html': 'no enforce-mode',
	'bad-meta-in-body.html': 'not a direct child of <head>',
	'bad-meta-in-comment.html': 'no enforce-mode',
	'bad-meta-in-noscript.html': 'not a direct child of <head>',
	'bad-no-meta.html': 'no enforce-mode',
	'bad-no-script-or-default.html': 'neither script-src nor default-src',
	'bad-nonce.html': "disallowed source 'nonce-abc123'",
	'bad-onattr-before-meta.html': 'has an inline handler (onerror)',
	'bad-report-only-mixed-case.html': 'no enforce-mode',
	'bad-report-only.html': 'no enforce-mode',
	'bad-noscript-before-meta.html': 'a <noscript> precedes the CSP meta tag',
	'bad-shape-c0-control.html': 'CSP policy shape: directive name not in svelte.config.js',
	'bad-shape-comma.html': 'CSP policy shape: value token',
	'bad-shape-double-space.html': 'CSP policy shape: empty directive or non-single-space',
	'bad-shape-script-src-attr-none.html': 'CSP policy shape: directive name not in svelte.config.js',
	'bad-shape-semicolon-no-space.html': 'CSP policy shape: value token',
	'bad-shape-tab-separator.html': 'CSP policy shape: directive name not in svelte.config.js',
	'bad-shape-unknown-directive.html': 'CSP policy shape: directive name not in svelte.config.js',
	'bad-shape-vt-before-name.html': 'CSP policy shape: directive name not in svelte.config.js',
	'bad-shape-vt-name-value.html': 'CSP policy shape: directive name not in svelte.config.js',
	'bad-scheme-data.html': 'script-src contains disallowed source data:',
	'bad-script-before-meta.html': 'a <script> precedes the CSP meta tag',
	'bad-script-src-attr-self.html': "script-src-attr must be absent or exactly 'none'",
	'bad-script-src-attr-unsafe.html': "script-src-attr contains disallowed source 'unsafe-inline'",
	'bad-script-src-elem.html': "script-src-elem contains disallowed source 'unsafe-inline'",
	'bad-utf16be-bom.html': 'non-UTF-8 byte order mark',
	'bad-utf16le-bom.html': 'non-UTF-8 byte order mark',
	'bad-two-metas.html': 'exactly one required',
	'bad-unsafe-eval.html': "script-src contains disallowed source 'unsafe-eval'",
	'bad-wildcard.html': 'script-src contains disallowed source *'
};

// Fixtures go in as BYTES, the same path the CLI takes (encoding rules included).
const read = (f) => readFileSync(join(dir, f));

test('positive control: a good fixture exists', () => {
	assert.ok(good.length > 0, 'no good fixture');
	assert.ok(files.includes('good.html'));
});

// Two-way reconciliation: a bad file with no recorded reason, or a reason
// whose file was deleted, both fail here.
test('REASON keys equal the set of bad-* fixtures', () => {
	assert.deepEqual(Object.keys(REASON).sort(), bad);
});

test('every fixture is named good* or bad-* (nothing silently unchecked)', () => {
	assert.deepEqual(
		files.filter((f) => !f.startsWith('good') && !f.startsWith('bad-')),
		[]
	);
});

for (const file of good) {
	test(`passes: ${file}`, () => assert.deepEqual(checkCspBytes(read(file)).errors, []));
}
for (const file of bad) {
	test(`fails: ${file}`, () => {
		const errors = checkCspBytes(read(file)).errors;
		const reason = REASON[file];
		assert.ok(
			errors.some((e) => e.includes(reason)),
			`${file} did not fail with "${reason}"; got ${JSON.stringify(errors)}`
		);
	});
}

// The CLI as a child process: the exit code and output are what CI sees.
// Each run gets its own copy in a fresh temp dir: the CLI writes csp-header.txt
// next to its input and must never touch the checked-in fixtures directory.
const stage = (fixture) => {
	const d = mkdtempSync(join(tmpdir(), 'csp-run-'));
	copyFileSync(join(dir, fixture), join(d, 'index.html'));
	return d;
};
const run = (script, fixture) => spawnSync(process.execPath, [script, join(stage(fixture), 'index.html')], { encoding: 'utf8' });

for (const file of bad) {
	test(`CLI exits non-zero: ${file}`, () => {
		const r = run(cli, file);
		assert.notEqual(r.status, 0, `exit ${r.status}; stdout ${r.stdout}`);
		assert.match(r.stderr, /check-csp: FAIL/);
	});
}

// The header file: exact bytes on a pass, nothing (and a stale one removed) on a fail.
const GOOD_HEADER =
	"default-src 'self'; script-src 'self' 'sha256-Z0IKutIsWS83KdlA1u8tyfrAKIOXeIMYbymW42PWnHY='; style-src 'self' 'unsafe-inline'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";
test('CLI writes csp-header.txt with the exact bytes for good.html (meta policy + frame-ancestors, no newline)', () => {
	const d = stage('good.html');
	const r = spawnSync(process.execPath, [cli, join(d, 'index.html')], { encoding: 'utf8' });
	assert.equal(r.status, 0, r.stderr);
	const bytes = readFileSync(join(d, 'csp-header.txt'));
	assert.deepEqual(bytes, Buffer.from(GOOD_HEADER, 'utf8'));
	assert.notEqual(bytes.at(-1), 0x0a);
});
test('checkCspBytes: header is null on failure, exact value on success; a UTF-8 BOM is accepted and not part of it', () => {
	assert.equal(checkCspBytes(read('good.html')).header, GOOD_HEADER);
	assert.equal(checkCspBytes(read('good-utf8-bom.html')).header, GOOD_HEADER);
	assert.equal(checkCspBytes(read('bad-wildcard.html')).header, null);
});
for (const file of bad) {
	test(`CLI writes no csp-header.txt and removes a stale one: ${file}`, () => {
		const d = stage(file);
		writeFileSync(join(d, 'csp-header.txt'), 'STALE');
		const r = spawnSync(process.execPath, [cli, join(d, 'index.html')], { encoding: 'utf8' });
		assert.notEqual(r.status, 0);
		assert.equal(existsSync(join(d, 'csp-header.txt')), false);
	});
}
test('CLI removes a stale csp-header.txt when the input file is missing, and writes none', () => {
	const d = mkdtempSync(join(tmpdir(), 'csp-run-'));
	writeFileSync(join(d, 'csp-header.txt'), 'STALE');
	const r = spawnSync(process.execPath, [cli, join(d, 'index.html')], { encoding: 'utf8' });
	assert.notEqual(r.status, 0);
	assert.equal(existsSync(join(d, 'csp-header.txt')), false);
});

test('CLI exits 0 and prints the ok line for good.html', () => {
	const r = run(cli, 'good.html');
	assert.equal(r.status, 0, r.stderr);
	assert.match(r.stdout, /^check-csp: ok \(/);
});

// Regression: an entry-point test comparing import.meta.url with argv[1] skipped
// main() and exited 0 when reached through a symlink.
test('CLI still fails a bad fixture when run through a symlinked path', () => {
	const link = join(mkdtempSync(join(tmpdir(), 'csp-link-')), 'check-csp.mjs');
	symlinkSync(cli, link);
	const bad = run(link, 'bad-no-meta.html');
	assert.notEqual(bad.status, 0, `symlinked run exited ${bad.status} on a bad fixture`);
	const good = run(link, 'good.html');
	assert.equal(good.status, 0, good.stderr);
	assert.match(good.stdout, /check-csp: ok/);
});

test('CLI fails closed on a missing file, naming the file and the cause', () => {
	const missing = join(mkdtempSync(join(tmpdir(), 'csp-run-')), 'does-not-exist.html');
	const r = spawnSync(process.execPath, [cli, missing], { encoding: 'utf8' });
	assert.notEqual(r.status, 0);
	assert.ok(r.stderr.includes('check-csp: FAIL'), r.stderr);
	assert.ok(r.stderr.includes(missing), r.stderr);
	assert.ok(r.stderr.includes('cannot read file'), r.stderr);
	assert.ok(r.stderr.includes('ENOENT'), r.stderr);
	assert.equal(r.stdout, '');
});

// Property: a policy padded with ANY non-ASCII character (whitespace-like or
// not) before the name or between name and value must fail; with ASCII
// whitespace (tab, LF, FF, CR, space) in the same places it must parse as the
// spec says, with no error.
const NON_ASCII = ['\u00a0', '\ufeff', '\u3000', '\u2003', '\u0085', '\u1680', '\u200b', '\u180e', '\u2028'];
const ASCII_WS = ['\t', '\n', '\f', '\r', ' '];
const page = (policy) =>
	`<!doctype html><html><head><meta http-equiv="content-security-policy" content="${policy}"></head><body></body></html>`;
for (const c of NON_ASCII) {
	const u = c.codePointAt(0).toString(16);
	test(`non-ASCII U+${u} padding fails (before name, between name and value, after value)`, () => {
		for (const policy of [`${c}script-src 'self'`, `script-src${c}'self'`, `script-src 'self'${c}`]) {
			const errors = checkCsp(page(policy));
			assert.ok(errors.some((e) => e.includes('not ASCII')), JSON.stringify([policy, errors]));
		}
	});
}
for (const c of ASCII_WS) {
	test(`ASCII whitespace ${JSON.stringify(c)} padding parses cleanly`, () => {
		const { directives, errors } = parsePolicy(`${c}SCRIPT-src${c}'self'${c};${c}`);
		assert.deepEqual(errors, []);
		assert.deepEqual([...directives], [['script-src', ["'self'"]]]);
	});
}
test('parsePolicy: first duplicate wins and is reported; comma is not a separator', () => {
	const { directives, errors } = parsePolicy("script-src 'self'; Script-Src 'unsafe-inline'; img-src a,b");
	assert.deepEqual([...directives], [['script-src', ["'self'"]], ['img-src', ['a,b']]]);
	assert.equal(errors.length, 1);
	assert.match(errors[0], /duplicate CSP directive script-src/);
});

// Shape allowlist (the emitted policy only). Property: a policy that passes the
// shape check, with ANY one control character U+0000..U+001F or U+007F inserted
// at ANY position, no longer passes. Covers VT (U+000B) and the rest of C0
// without listing which ones a browser happens to tolerate.
const GOOD_POLICY = "default-src 'self'; script-src 'self' 'sha256-Z0IKutIsWS83KdlA1u8tyfrAKIOXeIMYbymW42PWnHY='; object-src 'none'";
test('shape: the good policy passes', () => assert.deepEqual(shapeErrors(GOOD_POLICY), []));
test('shape: any control character at any position fails (incl. U+000B)', () => {
	// C0 and DEL.
	const controls = [...Array(0x20).keys(), 0x7f].map((c) => String.fromCharCode(c));
	assert.ok(controls.includes('\u000b'));
	for (const c of controls) {
		for (let i = 0; i <= GOOD_POLICY.length; i++) {
			const policy = GOOD_POLICY.slice(0, i) + c + GOOD_POLICY.slice(i);
			assert.notDeepEqual(shapeErrors(policy), [], JSON.stringify(policy));
		}
	}
});
test('shape: any non-ASCII character at any position fails', () => {
	// Incl. U+0085, a C1 control, in the value-token position.
	for (const c of [...NON_ASCII, '\u00e9', '\u{1f600}']) {
		for (let i = 0; i <= GOOD_POLICY.length; i++) {
			const policy = GOOD_POLICY.slice(0, i) + c + GOOD_POLICY.slice(i);
			assert.notDeepEqual(shapeErrors(policy), [], JSON.stringify(policy));
		}
	}
});
test('shape: non-single-space separators, trailing separators, comma and bare semicolon fail', () => {
	for (const policy of [
		"default-src  'self'", " default-src 'self'", "default-src 'self' ", "default-src\t'self'",
		"default-src 'self';", "default-src 'self'; ", "default-src 'self';;", "default-src 'self';script-src 'self'",
		"default-src 'self',script-src 'self'", '', "worker-src 'self'", "DEFAULT-SRC 'self'"
	]) {
		assert.notDeepEqual(shapeErrors(policy), [], JSON.stringify(policy));
	}
});
// The allowed directive names are derived from svelte.config.js, the config that
// emits the policy. Prove the derivation tracks it: every name the config sets
// is accepted. The count is pinned at nine as a deliberate tripwire: adding or
// removing a directive in the config fails here until this number is updated.
test('shape: directive names come from svelte.config.js kit.csp.directives', async () => {
	const { default: config } = await import(pathToFileURL(join(here, '..', 'svelte.config.js')).href);
	const names = Object.keys(config.kit.csp.directives);
	assert.equal(names.length, 9, names.join());
	for (const name of names) assert.deepEqual(shapeErrors(`${name} 'self'`), [], name);
});
