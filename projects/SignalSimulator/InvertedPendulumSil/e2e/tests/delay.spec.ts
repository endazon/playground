import { expect, test } from '@playwright/test';
import { boot, expectStaysBalanced, mode, push, readNumber, setRange, startFromUpright } from './helpers';

test.describe('通信遅延と遅延余裕', () => {
  test('帰還周期を変えるとバックが離散LQRを再設計し、遅延余裕が縮む', async ({ page }) => {
    await boot(page);
    const marginAt5ms = await readNumber(page.getByTestId('budget-margin'));
    const gainAt5ms = await page.getByTestId('metric-gain').textContent();

    // 「前回設計 100ms・片道20ms」プリセット
    await page.getByRole('button', { name: '前回設計 100ms・片道20ms' }).click();

    await expect(page.getByTestId('value-period')).toHaveText('100 ms');
    await expect(page.getByTestId('budget-margin')).toHaveText('59 ms');
    expect(await readNumber(page.getByTestId('budget-margin'))).toBeLessThan(marginAt5ms);
    expect(await page.getByTestId('metric-gain').textContent()).not.toBe(gainAt5ms);
  });

  test('振子を短くすると不安定極が上がり、遅延余裕が縮む', async ({ page }) => {
    await boot(page);
    expect(await readNumber(page.getByTestId('budget-margin'))).toBe(85);

    await setRange(page, 'slider-length', 0.3);

    await expect(page.getByTestId('metric-pole')).toHaveText(/7\.5\d rad\/s/);
    await expect(page.getByTestId('budget-margin')).toHaveText('57 ms');
  });

  test('遅延余裕の内側なら遅延を入れても倒立を維持する', async ({ page }) => {
    await boot(page);
    // 片道 25ms = 往復 50ms。理論遅延余裕 85ms の内側。
    await setRange(page, 'slider-latency', 25);
    await startFromUpright(page);

    await expect(page.getByTestId('budget-e2e')).not.toHaveText('—');
    expect(await readNumber(page.getByTestId('budget-e2e'))).toBeGreaterThan(40);
    await expectStaysBalanced(page, 6_000);
  });

  test('遅延余裕を超えると予算バーが危険域になり、外乱で転倒する', async ({ page }) => {
    await boot(page);
    // 片道 60ms = 往復 120ms。理論遅延余裕 85ms を超える。
    await setRange(page, 'slider-latency', 60);
    await startFromUpright(page);

    await expect(page.getByTestId('budget-fill')).toHaveAttribute('data-lv', 'danger');
    await expect(page.getByTestId('budget-note')).toContainText('理論余裕を超過');

    // 外乱を与えれば持ち堪えられない。
    // 遅れた指令で振動が育つため、先に振子が倒れるか台車がソフトリミットに達するかは
    // そのときのタイミング次第で、どちらもこの遅延では制御が破綻したことを意味する。
    await push(page, 'right');
    await expect(mode(page)).toHaveText(/FAULT/, { timeout: 40_000 });
    await expect(page.getByTestId('reason')).toHaveText(/転倒検知|ソフトリミット超過/);
  });

  test('予測器を有効にすると同じ遅延でも倒立を維持できる', async ({ page }) => {
    await boot(page);
    await setRange(page, 'slider-latency', 50); // 往復 100ms。補償なしでは余裕 85ms を超える。
    await page.getByTestId('toggle-predictor').check();
    await expect(page.getByTestId('budget-note')).toContainText('予測補償中');

    await startFromUpright(page);

    await expectStaysBalanced(page, 8_000, async (elapsed) => {
      if (elapsed > 3_000 && elapsed < 3_600) await push(page, 'right');
    });
  });
});
