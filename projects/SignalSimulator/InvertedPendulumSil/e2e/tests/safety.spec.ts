import { expect, test } from '@playwright/test';
import { boot, log, mode, reason, startFromUpright } from './helpers';

test.describe('監視の多重化', () => {
  test('非常停止でプラントが電圧を切り、バックが FAULT に落ちる', async ({ page }) => {
    await boot(page);
    await startFromUpright(page);
    await page.waitForTimeout(1_000);

    await page.getByTestId('btn-estop').click();

    await expect(mode(page)).toHaveText(/FAULT/);
    await expect(reason(page)).toContainText('非常停止');
    await expect(page.getByTestId('metric-volts')).toHaveText('0.00 V');

    // プラント側の即時通知と、バック側の状態遷移の両方がログに残る
    await expect(log(page)).toContainText('非常停止');
    await expect(log(page).locator('li.error').first()).toBeVisible();
  });

  test('FAULT 中は開始操作を受け付けない', async ({ page }) => {
    await boot(page);
    await startFromUpright(page);
    await page.getByTestId('btn-estop').click();
    await expect(mode(page)).toHaveText(/FAULT/);

    await expect(page.getByTestId('btn-swingup')).toBeDisabled();
    await expect(page.getByTestId('btn-balance')).toBeDisabled();
    await expect(page.getByTestId('btn-clear')).toBeEnabled();
  });

  test('異常リセットだけでは再開できず、プラント初期化が要る', async ({ page }) => {
    await boot(page);
    await startFromUpright(page);
    await page.waitForTimeout(500);
    await page.getByTestId('btn-estop').click();
    await expect(mode(page)).toHaveText(/FAULT/);

    // バック側の確認 (異常リセット) だけを行う
    await page.getByTestId('btn-clear').click();
    await expect(mode(page)).toHaveText(/IDLE/);

    // 現場のドライブは無効のままなので、次の運転セッションはプラントに拒否される。
    // (「倒立から開始」はプラント初期化を兼ねるので、ここでは純粋な開始操作を使う)
    await page.getByTestId('btn-swingup').click();
    await expect(mode(page)).toHaveText(/FAULT/);
    await expect(reason(page)).toContainText('ドライブ無効');
    await expect(log(page)).toContainText('ドライブ無効中の指令を拒否');

    // 現場側の復帰操作 (プラント初期化) を行えば再開できる
    await page.getByTestId('btn-clear').click();
    await page.getByTestId('btn-init').click();
    await page.getByTestId('btn-swingup').click();
    await expect(mode(page)).toHaveText(/SWINGUP/);
  });
});
