using Ip.Server.Hubs;
using Ip.Server.Sessions;

var builder = WebApplication.CreateBuilder(args);

// Blazor WASM クライアントの静的アセット (index.html を含む) は、既定では
// Development 環境でしか合成されない。Release で `dotnet run` したときに
// index.html が 404 になるのを防ぐため、環境によらず明示的に読み込む。
// publish 済みの構成ではマニフェストが存在しないので何も起きない。
builder.WebHost.UseStaticWebAssets();

builder.Services.AddSingleton<ControllerSessionManager>();
builder.Services
    .AddSignalR(options =>
    {
        // 制御ループが 5ms 周期で帰還を投げるため、既定のタイムアウトより短く設定して切断検知を早める
        options.ClientTimeoutInterval = TimeSpan.FromSeconds(10);
        options.KeepAliveInterval = TimeSpan.FromSeconds(3);
        options.EnableDetailedErrors = builder.Environment.IsDevelopment();
    })
    .AddMessagePackProtocol();

var app = builder.Build();

// SharedArrayBuffer / WasmEnableThreads を使う構成に切り替えるときだけ有効にする。
// COEP: require-corp は外部 CDN の読み込み条件も変えるため、既定では無効。
if (app.Configuration.GetValue("Sil:CrossOriginIsolation", false))
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
app.MapStaticAssets();

app.MapHub<SilHub>("/hub/sil");
app.MapGet("/healthz", (ControllerSessionManager sessions) => Results.Ok(new { status = "ok", sessions = sessions.Count }));
app.MapFallbackToFile("index.html");

app.Run();

/// <summary>結合テストから <c>WebApplicationFactory</c> で起動できるようにするための公開マーカー。</summary>
public partial class Program;
