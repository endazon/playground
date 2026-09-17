using Ip.Shared.Control;
using Ip.Shared.Model;
using Ip.Shared.Protocol;
using Ip.Shared.Simulation;

namespace Ip.Shared.Tests;

/// <summary>
/// コントローラとプラントを遅延線で結んだ、仮想時間のロックステップ結合試験台。
///
/// 実行環境のタイマ精度に依存しないため完全に再現性があり、
/// 「この遅延なら倒立が維持できるか」を CI で判定できる。
/// 実時間で動かすアプリ側 (Ip.Server / Ip.Client) と同じ <see cref="ControllerCore"/> と
/// <see cref="VirtualPlant"/> をそのまま使う点が重要で、テスト専用の簡略モデルは持ち込まない。
/// </summary>
public sealed class SilHarness
{
    private readonly DelayLine<object> _uplink;
    private readonly DelayLine<object> _downlink;
    private NetworkConfig _network;

    public SilHarness(
        NetworkConfig? network = null,
        ControlOptions? options = null,
        PlantConfig? plant = null,
        int seed = 12345)
    {
        _network = (network ?? new NetworkConfig()).Sanitized();
        var random = new Random(seed);

        var parameters = new PendulumParameters();
        if (plant is not null) parameters = parameters with { PendulumLength = plant.Sanitized().PendulumLength };

        Plant = new VirtualPlant(parameters, random);
        Controller = new ControllerCore();

        Plant.Configure(_network);
        Controller.SetNetwork(_network);
        if (plant is not null)
        {
            Plant.Configure(plant);
            Controller.SetPlant(plant);
        }
        if (options is not null) Controller.SetOptions(options);

        _uplink = new DelayLine<object>(DeliverToController, random);
        _downlink = new DelayLine<object>(DeliverToPlant, random);

        // プラント → 上り: 計測時刻を遅延の起点にする
        Plant.FeedbackReady += fb => { if (UplinkEnabled) _uplink.Push(fb, fb.TimestampMs, _network); };
        Plant.MotionEventRaised += ev => { if (UplinkEnabled) _uplink.Push(ev, ev.TimestampMs, _network); };

        // コントローラ → 下り: 送信時刻が起点
        Controller.VoltageProduced += cmd => { if (DownlinkEnabled) _downlink.Push(cmd, NowMs, _network); };
        Controller.AbortProduced += cmd => { if (DownlinkEnabled) _downlink.Push(cmd, NowMs, _network); };
        Controller.Logged += entry => Log.Add(entry);
    }

    public VirtualPlant Plant { get; }
    public ControllerCore Controller { get; }
    public List<LogEntry> Log { get; } = [];

    /// <summary>false にすると上り (帰還・イベント) が届かなくなる。通信途絶の試験用。</summary>
    public bool UplinkEnabled { get; set; } = true;

    /// <summary>false にすると下り (電圧指令) が届かなくなる。指令タイムアウトの試験用。</summary>
    public bool DownlinkEnabled { get; set; } = true;

    /// <summary>仮想時刻 [ms]。プラントとコントローラで共通 (同一ホスト相当、時計ズレなし)。</summary>
    public double NowMs { get; private set; }

    public void Configure(NetworkConfig network)
    {
        _network = network.Sanitized();
        Plant.Configure(_network);
        Controller.SetNetwork(_network);
    }

    /// <summary>1ms 進める。プラント積分 → 上り配送 → 制御計算 → 下り配送 → ウォッチドッグの順。</summary>
    public void Step()
    {
        NowMs += 1.0;
        Plant.AdvanceTo(NowMs);
        _uplink.Flush(NowMs);
        _downlink.Flush(NowMs);
        Controller.Tick(NowMs);
    }

    public void Run(double durationMs, Action<SilHarness>? onEachMillisecond = null)
    {
        double until = NowMs + durationMs;
        while (NowMs < until)
        {
            Step();
            onEachMillisecond?.Invoke(this);
        }
    }

    /// <summary>倒立から開始する (UI の「倒立から開始」相当: プラント初期化 → 少し待つ → 開始)。</summary>
    public void StartBalance()
    {
        Plant.Reset(upright: true);
        Run(30);
        Controller.Operate(OperatorAction.Balance, NowMs);
    }

    public void StartSwingUp()
    {
        Controller.Operate(OperatorAction.SwingUp, NowMs);
    }

    /// <summary>倒立を維持できたか。角度が転倒判定を超えず FAULT にも落ちていないこと。</summary>
    public bool IsBalancing =>
        Controller.Mode == ControlMode.Balance
        && Math.Abs(CartPoleDynamics.WrapAngle(Plant.State.Theta)) < ControllerCore.FallAngleRad;

    private void DeliverToController(object message)
    {
        switch (message)
        {
            case EncoderFeedback feedback:
                Controller.OnFeedback(feedback, NowMs);
                break;
            case MotionEvent motionEvent:
                Controller.OnMotionEvent(motionEvent, NowMs);
                break;
        }
    }

    private void DeliverToPlant(object message)
    {
        switch (message)
        {
            case VoltageCommand command:
                Plant.ApplyCommand(command, NowMs);
                break;
            case AbortCommand abort:
                Plant.ApplyAbort(abort);
                break;
        }
    }
}
