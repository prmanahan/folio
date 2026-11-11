// Build-time guard (Warden, #4710): the built SPA shell carries the page's
// script CSP as a <meta http-equiv> tag, so a build that loses the tag, moves
// it where the browser ignores it, or widens a script directive must fail CI.
// The rules live in csp-lib.mjs.
//
// On success it also writes csp-header.txt next to the checked file: the exact
// Content-Security-Policy HTTP header value (meta policy + frame-ancestors
// 'none', no trailing newline) that the server sends. On any failure it writes
// nothing and removes a stale csp-header.txt, so the server never serves a
// header the guard did not validate.
//
// Usage: node scripts/check-csp.mjs [path-to-index.html]   (default build/index.html)
//
// This file only runs main(): no "am I the entry point" test. Such a test
// (comparing import.meta.url with argv[1]) skipped main() and exited 0 when
// the script was reached through a symlinked path.
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { checkCspBytes } from './csp-lib.mjs';

const file = process.argv[2] ?? 'build/index.html';
const headerFile = join(dirname(file), 'csp-header.txt');

function fail(...messages) {
	try {
		rmSync(headerFile, { force: true });
	} catch (e) {
		console.error(`check-csp: FAIL (${headerFile}): cannot remove stale header file: ${e.message}`);
	}
	for (const message of messages) console.error(`check-csp: FAIL (${file}): ${message}`);
	process.exit(1);
}

let bytes;
try {
	bytes = readFileSync(file);
} catch (e) {
	fail(`cannot read file: ${e.message}`);
}
const { errors, header } = checkCspBytes(bytes);
if (errors.length > 0) {
	fail(...errors);
}
try {
	writeFileSync(headerFile, header);
} catch (e) {
	fail(`cannot write ${headerFile}: ${e.message}`);
}
console.log(`check-csp: ok (${file}); wrote ${headerFile}`);
