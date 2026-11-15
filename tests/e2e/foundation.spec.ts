import { expect, test } from '@playwright/test';

test('local fixture renders, accepts state changes and keeps navigation keyboard accessible', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('http://127.0.0.1:1420');
  await expect(page.getByRole('heading', { name: 'A little light' })).toBeVisible();
  await expect(page.getByText('Synthetic content · local testing only')).toBeVisible();
  await page.getByRole('button', { name: 'playing', exact: true }).click();
  await expect(page.getByText('Synthetic player · playing')).toBeVisible();
  await page.getByRole('button', { name: 'Library', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Your songs will live here' })).toBeVisible();
  await page.getByRole('button', { name: 'Return to Now' }).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByText('Synthetic player · playing')).toBeVisible();
  await page.screenshot({ path: 'artifacts/foundation-desktop.png', fullPage: true });
  await page.getByRole('button', { name: 'empty', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'A place for the words' })).toBeVisible();
  await page.setViewportSize({ width: 620, height: 480 });
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  expect(errors).toEqual([]);
});

test('production web assets do not silently enable the fixture or a remote app runtime', async ({ page }) => {
  await page.goto('http://127.0.0.1:1421');
  await expect(page.getByRole('heading', { name: 'Open Singalong on your desktop' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Fixture controls' })).toHaveCount(0);
});
