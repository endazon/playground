import { expect, test } from '@playwright/test';
import {
  boot, expect3dActive, expectNumberInRange, expectStaysBalanced, mode, push, readNumber, startFromUpright,
} from './helpers';

test.describe('基本動作', () => {
  test('起動するとコントローラに接続し、設計値を受け取る', async ({ page }) => {
    await boot(page);

    // バックが設計した離散 LQR ゲインと、倒立点の不安定極が届いている
    await expect(page.getByTestId('metric-pole')).toHaveText(/5\.3\d rad\/s/);
    const gains = (await page.getByTestId('metric-gain').textContent()) ?? '';
    expect(gains.split(',')).toHaveLength(4);

    // 既定条件 (振子長 0.6m / 帰還周期 5ms) の理論遅延余裕は 85ms
    expect(await readNumber(page.getByTestId('budget-margin'))).toBe(85);
  });

  test('3D ビューが読み込まれる', async ({ page }) => {
    // three.js は同梱している。読めなくなるとフォールバック表示に落ちるだけで
    // 他のテストは全部通ってしまうため、ここで明示的に固定する。
    await boot(page);
    await expect3dActive(page);
  });

  test('three.js を読めないときは側面図だけで動き続ける', async ({ page }) => {
    await page.route('**/three.min.js', route => route.abort());

    await boot(page);

    await expect(page.locator('#fallback3d')).toBeVisible();
    await expect(page.locator('#c2d')).toBeVisible();
    await startFromUpright(page);
    await expectStaysBalanced(page, 2_000);
  });

  test('倒立から開始すると倒立を維持する', async ({ page }) => {
    await boot(page);
    await startFromUpright(page);

    await expectStaysBalanced(page, 5_000);

    // 帰還が設定どおりの周期で届き、E2E 遅延が遅延余裕に対して十分小さい
    await expectNumberInRange(page.getByTestId('metric-feedback-interval'), 3, 9, '実測帰還周期');
    await expectNumberInRange(page.getByTestId('budget-e2e'), 0, 40, 'E2E 遅延');
    await expect(page.getByTestId('budget-fill')).toHaveAttribute('data-lv', 'ok');

    // 破棄された指令が無い = 世代管理が正常
    await expect(page.getByTestId('metric-command')).toHaveText('#1 / 0');
  });

  test('外乱を加えても倒立を維持する', async ({ page }) => {
    await boot(page);
    await startFromUpright(page);
    await page.waitForTimeout(1_000);

    await push(page, 'right');
    await page.waitForTimeout(1_500);
    await push(page, 'left');

    await expectStaysBalanced(page, 4_000);
  });

  test('停止すると IDLE に戻り、電圧が 0 になる', async ({ page }) => {
    await boot(page);
    await startFromUpright(page);
    await page.waitForTimeout(1_000);

    await page.getByTestId('btn-stop').click();

    await expect(mode(page)).toHaveText(/IDLE/);
    await expect(page.getByTestId('metric-volts')).toHaveText('0.00 V');
  });

  test('スイングアップで吊り下げ状態から倒立領域に入る', async ({ page }) => {
    await boot(page);

    await page.getByTestId('btn-swingup').click();

    await expect(mode(page)).toHaveText(/SWINGUP/);
    // エネルギー法で振り上げるので数秒かかる
    await expect(mode(page)).toHaveText(/BALANCE/, { timeout: 60_000 });
    await expectStaysBalanced(page, 3_000);
  });
});
