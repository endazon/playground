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

/** 「85 ms」「5.0 ms」のような表示から数値だけを取り出す。 */
export async function readNumber(locator: Locator): Promise<number> {
  const text = (await locator.textContent()) ?? '';
  const match = text.match(/-?\d+(\.\d+)?/);
  if (!match) throw new Error(`数値を読み取れない表示: "${text}"`);
  return Number(match[0]);
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
