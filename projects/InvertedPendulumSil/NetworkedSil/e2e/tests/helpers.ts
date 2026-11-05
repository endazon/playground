import { expect, type Locator, type Page } from '@playwright/test';

export const mode = (page: Page) => page.getByTestId('mode');
export const reason = (page: Page) => page.getByTestId('reason');
export const log = (page: Page) => page.getByTestId('log');

/** アプリを開き、コントローラに接続して IDLE になるまで待つ。 */
export async function boot(page: Page): Promise<void> {
  await page.goto('/');
  // Blazor WASM の起動 + SignalR 接続 + 初回の設計値配信までを待つ
  await expect(page.getByTestId('link'), 'コントローラに接続できていない').toHaveText(/接続済み/);
  await expect(mode(page)).toHaveText(/IDLE/);
  await expect(page.getByTestId('budget-margin')).not.toHaveText('—');
}

/** 「倒立から開始」。プラントを倒立姿勢に初期化してから開始指示が飛ぶ。 */
export async function startFromUpright(page: Page): Promise<void> {
  await page.getByTestId('btn-balance').click();
  await expect(mode(page)).toHaveText(/BALANCE/);
}

/** レンジスライダに値を設定する (Blazor の @oninput が発火する)。 */
export async function setRange(page: Page, testId: string, value: number | string): Promise<void> {
  await page.getByTestId(testId).fill(String(value));
}

/**
 * 数値表示が範囲に収まるまで待つ。`textContent()` の一発読みでは Playwright の
 * 自動リトライが効かず、100ms ごとに更新される EMA 表示を一瞬の値で掴んで落ちる。
 */
export async function expectNumberInRange(
  locator: Locator, min: number, max: number, label: string,
): Promise<void> {
  await expect
    .poll(async () => {
      const value = await tryReadNumber(locator);
      return Number.isNaN(value) ? null : value >= min && value <= max;
    }, { message: `${label} が ${min}〜${max} に入らない`, timeout: 25_000 })
    .toBe(true);
}

/** 3D ビューが生きているか (three.js を読めているか)。 */
export async function expect3dActive(page: Page): Promise<void> {
  await expect(page.locator('#fallback3d')).toBeHidden();
  await expect(page.locator('#c3d')).toBeVisible();
  expect(await page.evaluate(() => typeof (window as unknown as { THREE?: unknown }).THREE !== 'undefined'),
    'three.js が読み込めていない').toBe(true);
}

/** 「85 ms」「5.0 ms」のような表示から数値だけを取り出す。 */
export async function readNumber(locator: Locator): Promise<number> {
  const value = await tryReadNumber(locator);
  if (Number.isNaN(value)) throw new Error(`数値を読み取れない表示: "${await locator.textContent()}"`);
  return value;
}

/** 未受信の「—」表示では NaN を返す。ポーリングの中で例外を投げないため。 */
export async function tryReadNumber(locator: Locator): Promise<number> {
  const text = (await locator.textContent()) ?? '';
  const match = text.match(/-?\d+(\.\d+)?/);
  return match ? Number(match[0]) : Number.NaN;
}

/**
 * 指定時間のあいだ BALANCE を維持し続けることを確認する。
 * 「一瞬倒立したが直後に倒れた」を通してしまわないよう、定期的に状態を見る。
 */
export async function expectStaysBalanced(
  page: Page,
  durationMs: number,
  onTick?: (elapsedMs: number) => Promise<void>,
): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < durationMs) {
    await expect(mode(page), `${Date.now() - start}ms 時点で倒立を維持できていない`)
      .toHaveText(/BALANCE/, { timeout: 3_000 });
    await page.waitForTimeout(500);
    if (onTick) await onTick(Date.now() - start);
  }
}

/** 振子を左右に叩く。 */
export async function push(page: Page, direction: 'left' | 'right'): Promise<void> {
  await page.getByTestId(direction === 'left' ? 'btn-push-left' : 'btn-push-right').click();
}
