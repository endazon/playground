using Ip.Server.Hubs;
using Ip.Server.Sessions;

var builder = WebApplication.CreateBuilder(args);

// 制御ループのログは ms 単位の前後関係が重要なので、コンソールに時刻を付けて 1 行にまとめる。
builder.Logging.AddSimpleConsole(options =>
{
    options.SingleLine = true;
    options.TimestampFormat = "HH:mm:ss.fff ";
    options.UseUtcTimestamp = false;
});

// Blazor WASM クライアントの静的アセット (index.html を含む) は、既定では
// Development 環境でしか合成されない。Release で `dotnet run` したときに
// index.html が 404 になるのを防ぐため、環境によらず明示的に読み込む。
// publish 済みの構成ではマニフェストが存在しないので何も起きない。
builder.WebHost.UseStaticWebAssets();

builder.Services.AddSingleton<ControllerSessionManager>();
builder.Services.AddHostedService<SessionHeartbeat>();
builder.Services
    .AddSignalR(options =>
    {
        // 制御ループが 5ms 周期で帰還を投げるため、既定のタイムアウトより短く設定して切断検知を早める
        options.ClientTimeoutInterval = TimeSpan.FromSeconds(10);
        options.KeepAliveInterval = TimeSpan.FromSeconds(3);
        options.EnableDetailedErrors = builder.Environment.IsDevelopment();
        // このプロトコルの最大メッセージは 100 バイト未満。既定の 32KB を絞って増幅を防ぐ。
        options.MaximumReceiveMessageSize = 8 * 1024;
    })
    .AddMessagePackProtocol();

var app = builder.Build();

var startupLogger = app.Services.GetRequiredService<ILoggerFactory>().CreateLogger("Ip.Server.Startup");
bool crossOriginIsolation = app.Configuration.GetValue("Sil:CrossOriginIsolation", false);
startupLogger.LogInformation(
    "起動: 環境 {Environment}, MaxSessions={MaxSessions}, CrossOriginIsolation={Coi}, HeartbeatSeconds={Heartbeat}, ContentRoot={ContentRoot}",
    app.Environment.EnvironmentName,
    app.Configuration.GetValue("Sil:MaxSessions", 64),
    crossOriginIsolation,
    app.Configuration.GetValue("Sil:HeartbeatSeconds", 10),
    app.Environment.ContentRootPath);
startupLogger.LogInformation(
    "ログレベル: Default={Default}, Ip={Ip}, Ip.Controller={Controller}, Ip.Downlink={Downlink} (帰還 1 本ごとの内容は Ip.Controller / Ip.Server.Hubs.SilHub を Trace にすると出る)",
    app.Configuration["Logging:LogLevel:Default"] ?? "(未設定)",
    app.Configuration["Logging:LogLevel:Ip"] ?? "(未設定)",
    app.Configuration["Logging:LogLevel:Ip.Controller"] ?? "(Ip を継承)",
    app.Configuration["Logging:LogLevel:Ip.Downlink"] ?? "(Ip を継承)");

app.Lifetime.ApplicationStarted.Register(() =>
    startupLogger.LogInformation("待ち受け開始: {Urls} — Hub は /hub/sil, ヘルスチェックは /healthz", string.Join(", ", app.Urls)));
app.Lifetime.ApplicationStopping.Register(() =>
    startupLogger.LogInformation("停止要求を受けた: セッション {Count} 件を破棄する",
        app.Services.GetRequiredService<ControllerSessionManager>().Count));
app.Lifetime.ApplicationStopped.Register(() => startupLogger.LogInformation("停止完了"));

// SharedArrayBuffer / WasmEnableThreads を使う構成に切り替えるときだけ有効にする。
// COEP: require-corp は外部 CDN の読み込み条件も変えるため、既定では無効。
if (crossOriginIsolation)
{
    app.Use(async (context, next) =>
    {
        context.Response.Headers["Cross-Origin-Opener-Policy"] = "same-origin";
        context.Response.Headers["Cross-Origin-Embedder-Policy"] = "require-corp";
        await next();
    });
}

// Blazor WASM のフレームワークファイル (_framework/*) は指紋付きで配信されるため、
// ファイル名で引く UseStaticFiles ではなく、マニフェスト駆動の MapStaticAssets を使う。
// 最低限のセキュリティヘッダ。Blazor WASM には 'wasm-unsafe-eval' が必須で、
// スコープ付き CSS のため style-src に 'unsafe-inline' が要る。
app.Use(async (context, next) =>
{
    var headers = context.Response.Headers;
    headers["X-Content-Type-Options"] = "nosniff";
    headers["Content-Security-Policy"] =
        "default-src 'self'; " +
        "script-src 'self' 'wasm-unsafe-eval'; " +
        "style-src 'self' 'unsafe-inline'; " +
        "img-src 'self' data:; " +
        "connect-src 'self' ws: wss:; " +
        "object-src 'none'; base-uri 'self'; frame-ancestors 'none'";
    await next();
});

app.MapStaticAssets();

app.MapHub<SilHub>("/hub/sil");
app.MapGet("/healthz", (ControllerSessionManager sessions) => Results.Ok(new
{
    status = "ok",
    sessions = sessions.Count,
    maxSessions = sessions.MaxSessions,
    totalCreated = sessions.TotalCreated,
    totalRejected = sessions.TotalRejected,
    totalFaulted = sessions.TotalFaulted,
}));
app.MapFallbackToFile("index.html");

app.Run();

/// <summary>結合テストから <c>WebApplicationFactory</c> で起動できるようにするための公開マーカー。</summary>
public partial class Program;
