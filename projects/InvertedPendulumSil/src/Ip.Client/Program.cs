using Ip.Client;
using Ip.Client.Services;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

// ブラウザの console へ出すログの絞り込み。wwwroot/appsettings.json (と appsettings.{環境}.json) の
// Logging セクションで、カテゴリ (Ip.Client.Services.PlantRunner / Ip.Plant / Ip.Uplink) ごとに変えられる。
// 帰還 1 本ごとの内容は Ip.Plant を Trace にすると出る (数百行/秒になるので既定では出さない)。
builder.Logging.AddConfiguration(builder.Configuration.GetSection("Logging"));

// プラントは接続 1 本につき 1 台。アプリ全体で共有する。
builder.Services.AddSingleton<PlantRunner>();

var host = builder.Build();

var logger = host.Services.GetRequiredService<ILoggerFactory>().CreateLogger("Ip.Client.Startup");
logger.LogInformation("Blazor WASM 起動: 環境 {Environment}, BaseAddress {BaseAddress}, ログ既定 {Default} / Ip {Ip} / Ip.Plant {Plant}",
    builder.HostEnvironment.Environment, builder.HostEnvironment.BaseAddress,
    builder.Configuration["Logging:LogLevel:Default"] ?? "(未設定)",
    builder.Configuration["Logging:LogLevel:Ip"] ?? "(未設定)",
    builder.Configuration["Logging:LogLevel:Ip.Plant"] ?? "(Ip を継承)");

await host.RunAsync();
