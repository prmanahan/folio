import { test, expect, type Page } from '@playwright/test';

// ─── Shared mock helpers ────────────────────────────────────────────────────

/**
 * Mock all the data endpoints the page needs so it renders without a real
 * backend. Returns a minimal profile so the page loads past the "Loading..."
 * skeleton.
 */
async function mockDataEndpoints(page: Page) {
	await page.route('/api/profile', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				name: 'Peter Manahan',
				email: 'peter@example.com',
				title: 'Software Architect',
				location: 'Remote',
				phone: '',
				linkedin_url: '',
				github_url: '',
				twitter_url: '',
				elevator_pitch: 'Test pitch.',
				availability_status: 'open',
				availability_date: '',
				remote_preference: 'remote',
			}),
		})
	);
	await page.route('/api/links', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
	);
	await page.route('/api/skills', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
	);
	await page.route('/api/experience', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
	);
	await page.route('/api/education', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: '[]' })
	);
	await page.route('/api/faq/suggestions', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{ id: 1, question: 'What is your experience with distributed systems?' },
				{ id: 2, question: 'What languages do you know?' },
			]),
		})
	);
}

/**
 * Mock the /api/chat SSE endpoint to stream a fixed response.
 * Sends tokens one at a time then a [DONE] sentinel.
 */
async function mockChatEndpoint(page: Page, responseTokens: string[]) {
	await page.route('/api/chat', async (route) => {
		const sseBody = responseTokens.map((t) => `data: ${t}\n`).join('') + 'data: [DONE]\n';
		await route.fulfill({
			status: 200,
			contentType: 'text/event-stream',
			body: sseBody,
		});
	});
}

/**
 * Mock /api/chat to return a 500 error.
 */
async function mockChatError(page: Page) {
	await page.route('/api/chat', (route) =>
		route.fulfill({ status: 500, body: 'Internal Server Error' })
	);
}

/**
 * Mock /api/fit to return a canned FitVerdict.
 */
async function mockFitEndpoint(page: Page) {
	await page.route('/api/fit', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				verdict: 'strong_fit',
				headline: 'Strong match for this role',
				opening: 'Peter is an excellent candidate.',
				gaps: [
					{
						requirement: 'Kubernetes',
						gap_title: 'Limited hands-on k8s',
						explanation: 'Some exposure but not deep.',
					},
				],
				transfers: [
					{
						skill: 'Distributed Systems',
						relevance: 'Directly applicable to the platform team.',
					},
				],
				recommendation: 'Recommend moving forward.',
			}),
		})
	);
}

// ─── Tests ──────────────────────────────────────────────────────────────────

test.describe('AI Pane', () => {
	test.beforeEach(async ({ page }) => {
		await mockDataEndpoints(page);
		await page.goto('/');
		// Wait for the page to move past the loading skeleton
		await page.waitForSelector('text=Peter Manahan');
	});

	// ── Open / close ──────────────────────────────────────────────────────

	test('opens when Ask AI button is clicked', async ({ page }) => {
		const pane = page.locator('.ai-pane');
		// Pane exists in DOM but is slid off-screen initially
		await expect(pane).not.toHaveClass(/open/);

		await page.getByRole('button', { name: 'Ask AI' }).click();

		await expect(pane).toHaveClass(/open/);
	});

	test('closes when the close button is clicked', async ({ page }) => {
		await page.getByRole('button', { name: 'Ask AI' }).click();
		const pane = page.locator('.ai-pane');
		await expect(pane).toHaveClass(/open/);

		await page.getByRole('button', { name: 'Close AI pane' }).click();
		await expect(pane).not.toHaveClass(/open/);
	});

	test('closes when Escape key is pressed', async ({ page }) => {
		await page.getByRole('button', { name: 'Ask AI' }).click();
		const pane = page.locator('.ai-pane');
		await expect(pane).toHaveClass(/open/);

		await page.keyboard.press('Escape');
		await expect(pane).not.toHaveClass(/open/);
	});

	// ── Tab switching ─────────────────────────────────────────────────────

	test('defaults to Chat tab', async ({ page }) => {
		await page.getByRole('button', { name: 'Ask AI' }).click();
		// The Chat tab button should be active
		const chatTab = page.getByRole('button', { name: 'Chat' });
		await expect(chatTab).toHaveClass(/active/);
		// Textarea (chat input) should be visible
		await expect(page.locator('textarea')).toBeVisible();
	});

	test('switches to Job Fit tab', async ({ page }) => {
		await page.getByRole('button', { name: 'Ask AI' }).click();
		await page.getByRole('button', { name: 'Job Fit' }).click();

		const jobFitTab = page.getByRole('button', { name: 'Job Fit' });
		await expect(jobFitTab).toHaveClass(/active/);

		// The job description textarea should be visible
		await expect(page.getByLabel('Paste a job description')).toBeVisible();
	});

	test('can switch back from Job Fit to Chat', async ({ page }) => {
		await page.getByRole('button', { name: 'Ask AI' }).click();
		await page.getByRole('button', { name: 'Job Fit' }).click();
		await page.getByRole('button', { name: 'Chat' }).click();

		await expect(page.getByRole('button', { name: 'Chat' })).toHaveClass(/active/);
		await expect(page.locator('textarea[placeholder="Type your transmission..."]')).toBeVisible();
	});

	// ── Empty state & suggested questions ────────────────────────────────

	test('shows empty state with suggested questions when no messages exist', async ({ page }) => {
		await page.getByRole('button', { name: 'Ask AI' }).click();

		await expect(page.getByText('Ask me anything')).toBeVisible();
		// FAQ suggestions are loaded async; wait for them
		await expect(page.getByText('What is your experience with distributed systems?')).toBeVisible({
			timeout: 5000,
		});
		await expect(page.getByText('What languages do you know?')).toBeVisible({ timeout: 5000 });
	});

	test('clicking a suggested question sends it as a message', async ({ page }) => {
		await mockChatEndpoint(page, ['Test', ' response']);
		await page.getByRole('button', { name: 'Ask AI' }).click();

		await page.getByText('What languages do you know?').click();

		// The user message should appear
		await expect(page.getByText('What languages do you know?')).toBeVisible();
	});

	// ── Chat flow (mocked SSE) ────────────────────────────────────────────

	test('user message appears immediately after sending', async ({ page }) => {
		await mockChatEndpoint(page, ['Hi', ' there!']);
		await page.getByRole('button', { name: 'Ask AI' }).click();

		const textarea = page.locator('textarea[placeholder="Type your transmission..."]');
		await textarea.fill('Hello from test');
		await page.getByRole('button', { name: 'Transmit message' }).click();

		await expect(page.getByText('Hello from test')).toBeVisible();
	});

	test('assistant response streams in from mocked SSE', async ({ page }) => {
		await mockChatEndpoint(page, ['Hi', ' there!']);
		await page.getByRole('button', { name: 'Ask AI' }).click();

		const textarea = page.locator('textarea[placeholder="Type your transmission..."]');
		await textarea.fill('Hello');
		await page.getByRole('button', { name: 'Transmit message' }).click();

		// Eventually the full streamed response should appear
		await expect(page.getByText('Hi there!')).toBeVisible({ timeout: 5000 });
	});

	test('typing indicator (spinner) disappears after stream completes', async ({ page }) => {
		await mockChatEndpoint(page, ['Done']);
		await page.getByRole('button', { name: 'Ask AI' }).click();

		const textarea = page.locator('textarea[placeholder="Type your transmission..."]');
		await textarea.fill('Test');
		await page.getByRole('button', { name: 'Transmit message' }).click();

		// After stream ends, the Transmit button text should return (spinner gone)
		await expect(page.getByRole('button', { name: 'Transmit message' })).toContainText(
			'Transmit',
			{ timeout: 5000 }
		);
		// Spinner element should not be visible
		await expect(page.locator('.spinner')).not.toBeVisible();
	});

	test('Enter key sends the message', async ({ page }) => {
		await mockChatEndpoint(page, ['Ack']);
		await page.getByRole('button', { name: 'Ask AI' }).click();

		const textarea = page.locator('textarea[placeholder="Type your transmission..."]');
		await textarea.fill('Enter key test');
		await textarea.press('Enter');

		await expect(page.getByText('Enter key test')).toBeVisible();
	});

	test('Shift+Enter does not send the message', async ({ page }) => {
		await page.getByRole('button', { name: 'Ask AI' }).click();

		const textarea = page.locator('textarea[placeholder="Type your transmission..."]');
		await textarea.fill('Line one');
		await textarea.press('Shift+Enter');

		// Pane should still be open and no message in the messages list
		await expect(page.locator('.ai-pane')).toHaveClass(/open/);
		// Textarea still has content (message was not sent)
		await expect(textarea).toHaveValue(/Line one/);
	});

	// ── Chat error handling ───────────────────────────────────────────────

	test('shows error message on 500 response from /api/chat', async ({ page }) => {
		await mockChatError(page);
		await page.getByRole('button', { name: 'Ask AI' }).click();

		const textarea = page.locator('textarea[placeholder="Type your transmission..."]');
		await textarea.fill('Trigger error');
		await page.getByRole('button', { name: 'Transmit message' }).click();

		await expect(page.getByRole('alert')).toContainText('AI features are currently unavailable', {
			timeout: 5000,
		});
	});

	// ── Job Fit flow (mocked) ─────────────────────────────────────────────

	test('Job Fit: verdict badge, headline, gaps, and transfers render', async ({ page }) => {
		await mockFitEndpoint(page);
		await page.getByRole('button', { name: 'Ask AI' }).click();
		await page.getByRole('button', { name: 'Job Fit' }).click();

		const jdTextarea = page.getByLabel('Paste a job description');
		await jdTextarea.fill('We are looking for a senior backend engineer with Kubernetes experience.');

		await page.getByRole('button', { name: 'Analyze Fit' }).click();

		// Verdict badge
		await expect(page.getByText('Strong Fit')).toBeVisible({ timeout: 5000 });
		// Headline
		await expect(page.getByText('Strong match for this role')).toBeVisible();
		// Gap
		await expect(page.getByText('Kubernetes')).toBeVisible();
		await expect(page.getByText('Limited hands-on k8s')).toBeVisible();
		// Transfer
		await expect(page.getByText('Distributed Systems')).toBeVisible();
		await expect(page.getByText('Directly applicable to the platform team.')).toBeVisible();
	});

	test('Job Fit: "Try Another" resets the form', async ({ page }) => {
		await mockFitEndpoint(page);
		await page.getByRole('button', { name: 'Ask AI' }).click();
		await page.getByRole('button', { name: 'Job Fit' }).click();

		await page.getByLabel('Paste a job description').fill('Some job description here.');
		await page.getByRole('button', { name: 'Analyze Fit' }).click();

		// Wait for result
		await expect(page.getByText('Strong Fit')).toBeVisible({ timeout: 5000 });

		await page.getByRole('button', { name: 'Try Another' }).click();

		// Should show textarea again
		await expect(page.getByLabel('Paste a job description')).toBeVisible();
	});

	test('Job Fit: Analyze Fit button disabled when textarea is empty', async ({ page }) => {
		await page.getByRole('button', { name: 'Ask AI' }).click();
		await page.getByRole('button', { name: 'Job Fit' }).click();

		await expect(page.getByRole('button', { name: 'Analyze Fit' })).toBeDisabled();
	});

	test('Job Fit: shows error on API failure', async ({ page }) => {
		await page.route('/api/fit', (route) =>
			route.fulfill({ status: 500, body: 'Internal Server Error' })
		);
		await page.getByRole('button', { name: 'Ask AI' }).click();
		await page.getByRole('button', { name: 'Job Fit' }).click();

		await page.getByLabel('Paste a job description').fill('Some description');
		await page.getByRole('button', { name: 'Analyze Fit' }).click();

		await expect(page.getByRole('alert')).toContainText('AI features are currently unavailable', {
			timeout: 5000,
		});
	});
});

// ─── Error event and empty stream tests ─────────────────────────────────────

test.describe('Chat error recovery', () => {
	test('shows error when backend sends SSE error event', async ({ page }) => {
		await mockDataEndpoints(page);

		// Mock /api/chat to return 200 with an error event in the SSE stream
		await page.route('/api/chat', (route) =>
			route.fulfill({
				status: 200,
				contentType: 'text/event-stream',
				body: 'event: error\ndata: AI response failed. Please try again.\n\ndata: [DONE]\n\n',
			})
		);

		await page.goto('/');
		await page.getByRole('button', { name: 'Ask AI' }).click();
		await page.getByPlaceholder('Type your transmission').fill('hello');
		await page.getByRole('button', { name: 'Transmit' }).click();

		await expect(page.getByRole('alert')).toContainText('AI response failed', {
			timeout: 5000,
		});
	});

	test('shows error when backend sends empty stream (only DONE)', async ({ page }) => {
		await mockDataEndpoints(page);

		// Mock /api/chat to return 200 with just [DONE] and no content
		await page.route('/api/chat', (route) =>
			route.fulfill({
				status: 200,
				contentType: 'text/event-stream',
				body: 'data: [DONE]\n\n',
			})
		);

		await page.goto('/');
		await page.getByRole('button', { name: 'Ask AI' }).click();
		await page.getByPlaceholder('Type your transmission').fill('hello');
		await page.getByRole('button', { name: 'Transmit' }).click();

		await expect(page.getByRole('alert')).toContainText('AI response was empty', {
			timeout: 5000,
		});
	});
});
