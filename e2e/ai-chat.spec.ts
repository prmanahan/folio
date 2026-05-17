import { test, expect } from '@playwright/test';
import { mockPublicApi, PROFILE_NAME } from './fixtures';

test.describe('AI chat pane', () => {
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
    await page.goto('/');
    // Wait for hub to load
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({ timeout: 10000 });
  });

  test('Ask AI card exists on home page', async ({ page }) => {
    await expect(page.getByRole('button', { name: /ask ai/i })).toBeVisible();
  });

  test('AI pane opens when Ask AI card is clicked', async ({ page }) => {
    await page.getByRole('button', { name: /ask ai/i }).click();
    // Pane should slide in — look for the tab structure
    await expect(page.getByRole('button', { name: /chat/i }).first()).toBeVisible({ timeout: 5000 });
  });

  test('Chat and Job Fit tabs are present when pane is open', async ({ page }) => {
    await page.getByRole('button', { name: /ask ai/i }).click();
    await expect(page.getByRole('button', { name: /chat/i }).first()).toBeVisible({ timeout: 5000 });
    await expect(page.getByRole('button', { name: /job fit/i })).toBeVisible();
  });

  test('chat input field is visible', async ({ page }) => {
    await page.getByRole('button', { name: /ask ai/i }).click();
    const textarea = page.getByPlaceholder(/type your transmission/i);
    await expect(textarea).toBeVisible({ timeout: 5000 });
  });

  test('sending a message does not crash the app', async ({ page }) => {
    await page.getByRole('button', { name: /ask ai/i }).click();

    const textarea = page.getByPlaceholder(/type your transmission/i);
    await expect(textarea).toBeVisible({ timeout: 5000 });
    await textarea.fill('What is Peter\'s background?');
    await page.getByRole('button', { name: /transmit message/i }).click();

    await page.waitForTimeout(2000);

    // App should not crash: hub content still present
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible();

    // Either a response is streaming OR an error message is shown — both are acceptable
    const hasError = await page.locator('[role="alert"]').isVisible();
    const hasUserMessage = await page.getByText("What is Peter's background?").isVisible();
    expect(hasError || hasUserMessage).toBeTruthy();
  });

  test('pane closes with close button', async ({ page }) => {
    await page.getByRole('button', { name: /ask ai/i }).click();
    const aiPane = page.locator('.ai-pane');
    await expect(aiPane).toHaveClass(/open/, { timeout: 5000 });

    await page.getByRole('button', { name: /close ai pane/i }).click();
    await expect(aiPane).not.toHaveClass(/open/, { timeout: 3000 });
  });

  test('pane closes with Escape key', async ({ page }) => {
    await page.getByRole('button', { name: /ask ai/i }).click();
    const aiPane = page.locator('.ai-pane');
    await expect(aiPane).toHaveClass(/open/, { timeout: 5000 });

    await page.keyboard.press('Escape');
    await expect(aiPane).not.toHaveClass(/open/, { timeout: 3000 });
  });
});

/**
 * RED-PHASE e2e acceptance — spec #572 Task 5, R19/R20, Scenarios 8/9/15.
 *
 * Mock seam (FRONTEND STUB at the network boundary — documented for task 981
 * / Rune; implement against the SAME seam): the dev server used by the
 * Playwright `webServer` has no backend, so we intercept `**\/api\/chat`
 * with `page.route(...)` and fulfill a synthetic `text/event-stream` body
 * carrying each new terminal event. The real frontend SSE parser + chat
 * components run end-to-end against that body. Backend mock-Anthropic-HTTP
 * harness is NOT reachable from the dev server — the spec explicitly permits
 * the frontend stub fallback for the e2e surface.
 *
 * Contract asserted (visual latitude per R20 — assert treatment class, not a
 * specific new class name beyond what the component already uses for errors):
 *   - NEGATIVE (load-bearing): the canned message MUST NOT render inside the
 *     `.assistant-bubble` (AI-bubble styling).
 *   - POSITIVE: it IS surfaced via the error/notice treatment the component
 *     already uses (`[role="alert"]` / `.error-message`).
 */
test.describe('AI chat — new typed terminal events (spec #572 R19/R20)', () => {
  const REFUSAL_MSG = 'The assistant declined to respond to that request.';
  const CONTEXT_MSG =
    "The request exceeded the model's context window. Try a shorter input.";
  const TRUNCATED_MSG = 'The response was cut off due to length.';

  // Mirror the working setup of the pre-existing chat suite: without the
  // public-API mock + a settled home page, the "Ask AI" trigger never
  // mounts and openChatAndSend times out at the click (an infra failure,
  // NOT the behavioral red). This is the same proven seam, not an
  // assertion change.
  test.beforeEach(async ({ page }) => {
    await mockPublicApi(page);
    await page.goto('/');
    await expect(page.getByRole('heading', { name: PROFILE_NAME })).toBeVisible({
      timeout: 10000,
    });
  });

  async function stubChat(page: import('@playwright/test').Page, sseBody: string) {
    await page.route('**/api/chat', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: sseBody,
      })
    );
  }

  async function openChatAndSend(page: import('@playwright/test').Page) {
    await page.getByRole('button', { name: /ask ai/i }).click();
    const textarea = page.getByPlaceholder(/type your transmission/i);
    await expect(textarea).toBeVisible({ timeout: 5000 });
    await textarea.fill('tell me about Peter');
    await page.getByRole('button', { name: /transmit message/i }).click();
  }

  test('refusal renders as notice, NOT as an AI content bubble (R20 / Scenario 8)', async ({
    page,
  }) => {
    await stubChat(page, `event: refusal\ndata: ${REFUSAL_MSG}\ndata: [DONE]\n`);
    await openChatAndSend(page);

    // Positive: surfaced via the error/notice treatment
    await expect(page.locator('[role="alert"]')).toHaveText(REFUSAL_MSG, {
      timeout: 5000,
    });
    // Negative (load-bearing): NOT inside the AI content bubble
    await expect(page.locator('.assistant-bubble', { hasText: REFUSAL_MSG })).toHaveCount(
      0
    );
  });

  test('context_exceeded renders as notice, NOT as an AI content bubble (R20 / Scenario 9)', async ({
    page,
  }) => {
    await stubChat(page, `event: context_exceeded\ndata: ${CONTEXT_MSG}\ndata: [DONE]\n`);
    await openChatAndSend(page);

    await expect(page.locator('[role="alert"]')).toHaveText(CONTEXT_MSG, {
      timeout: 5000,
    });
    await expect(
      page.locator('.assistant-bubble', { hasText: CONTEXT_MSG })
    ).toHaveCount(0);
  });

  test('truncated: pre-truncation content renders, notice text NOT in the bubble (R19 / Scenario 15)', async ({
    page,
  }) => {
    await stubChat(
      page,
      `data: Peter ships reliable systems.\nevent: truncated\ndata: ${TRUNCATED_MSG}\ndata: [DONE]\n`
    );
    await openChatAndSend(page);

    // Pre-truncation content still renders in the AI bubble
    await expect(
      page.locator('.assistant-bubble', { hasText: 'Peter ships reliable systems.' })
    ).toHaveCount(1, { timeout: 5000 });
    // The truncated canned notice is surfaced via the notice treatment
    await expect(page.locator('[role="alert"]')).toHaveText(TRUNCATED_MSG, {
      timeout: 5000,
    });
    // ...and is NOT polluting the AI content bubble
    await expect(
      page.locator('.assistant-bubble', { hasText: TRUNCATED_MSG })
    ).toHaveCount(0);
  });
});
