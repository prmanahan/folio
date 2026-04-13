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
