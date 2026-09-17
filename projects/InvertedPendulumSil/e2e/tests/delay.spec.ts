import { expect, test } from '@playwright/test';
import {
  boot, expectNumberInRange, expectStaysBalanced, mode, push, readNumber, setRange, startFromUpright,
} from './helpers';

test.describe('通信遅延と遅延余裕', () => {
  test('帰還周期を変えるとバックが離散LQRを再設計し、遅延余裕が縮む', async ({ page }) => {
    await boot(page);
    const marginAt5ms = await readNumber(page.getByTestId('budget-margin'));
    const gainAt5ms = await page.getByTestId('metric-gain').textContent();

    // 「前回設計」プリセット (表示文言ではなく testid で選ぶ)
    await page.getByTestId('preset-legacy').click();

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
    await expectNumberInRange(page.getByTestId('budget-e2e'), 40, 90, '注入した遅延');
    await expectStaysBalanced(page, 6_000);
  });

  test('遅延余裕を超えると予算バーが危険域になり、外乱で転倒する', async ({ page }) => {
    await boot(page);
    // 片道 60ms = 往復 120ms。理論遅延余裕 85ms を超える。
    await setRange(page, 'slider-latency', 60);
    // ここでは BALANCE に入ったことを待たない。余裕を超えているので、
    // 10Hz の画面更新に BALANCE が 1 度も映らないまま倒れることがある。
    await page.getByTestId('btn-balance').click();

    // 危険域の表示は運転中しか出ない。自励振動で先に落ちるとこの 2 つは永久に一致しないので、
    // 「危険域を観測する」か「FAULT に落ちる」かのどちらかが起きれば主張は満たされている。
    const sawDanger = await page
      .waitForFunction(
        () => document.querySelector('[data-testid="budget-fill"]')?.getAttribute('data-lv') === 'danger'
          || !!document.querySelector('[data-testid="mode"]')?.textContent?.includes('FAULT'),
        undefined, { timeout: 25_000 })
      .then(() => page.getByTestId('budget-fill').getAttribute('data-lv'))
      .then(level => level === 'danger', () => false);
    if (sawDanger) await expect(page.getByTestId('budget-note')).toContainText('理論余裕を超過');

    // 外乱を与えれば持ち堪えられない。
    // 遅れた指令で振動が育つため、先に振子が倒れるか台車がソフトリミットに達するかは
    // そのときのタイミング次第で、どちらもこの遅延では制御が破綻したことを意味する。
    await push(page, 'right');
    await expect(mode(page)).toHaveText(/FAULT/, { timeout: 40_000 });
    await expect(page.getByTestId('reason')).toHaveText(/転倒検知|ソフトリミット超過/);
  });

  /*
   * 「予測器で耐えられる遅延がどこまで伸びるか」という量的な主張は、
   * 仮想時間で完全に再現できる DelayedLoopIntegrationTests.Predictor_ExtendsTheUsableDelay が持つ。
   * ここでブラウザ実時間で境界ぎりぎりを試すと、共有 CI の負荷で結果が揺れて
   * 「制御が悪いのか環境が遅いのか」を区別できないテストになる。
   * したがって e2e では「設定がバックまで届き、制御を壊さない」ことだけを固定する。
   */
  test('予測器を有効にしても遅延下で倒立を維持する', async ({ page }) => {
    await boot(page);
    await setRange(page, 'slider-latency', 35);
    await page.getByTestId('toggle-predictor').check();
    await expect(page.getByTestId('budget-note')).toContainText('予測補償中');

    await startFromUpright(page);

    // 外乱は「経過時間の窓」ではなく明示的に 1 回入れる。
    // 窓に頼ると、ループ 1 周の実時間が伸びた環境で外乱が 0 回になり、
    // 外乱に耐えることの検証が消えたまま緑になる。
    await expectStaysBalanced(page, 3_000);
    await push(page, 'right');
    await expectStaysBalanced(page, 4_000);
  });

  test('ジッタを入れると E2E 遅延が伸びる', async ({ page }) => {
    await boot(page);
    // 片道 10ms + ジッタ 0〜20ms。往復でも理論遅延余裕 85ms の内側に収まる範囲。
    await setRange(page, 'slider-latency', 10);
    await setRange(page, 'slider-jitter', 20);
    await startFromUpright(page);

    await expectNumberInRange(page.getByTestId('budget-e2e'), 22, 80, 'ジッタ込みの E2E 遅延');
  });

  test('エンコーダノイズを入れても倒立を維持する', async ({ page }) => {
    await boot(page);
    await setRange(page, 'slider-noise', 2);
    await startFromUpright(page);

    await expectStaysBalanced(page, 6_000);
  });

  test('台車の目標位置を動かすと、そこへランプで寄る', async ({ page }) => {
    await boot(page);
    await startFromUpright(page);
    await expectStaysBalanced(page, 1_500);

    await setRange(page, 'slider-target', 0.3);

    // 一足飛びではなくランプで寄るので数秒かかる
    await expectStaysBalanced(page, 6_000);
    await expect(mode(page)).toHaveText(/BALANCE/);
  });
});
