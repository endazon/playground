using Ip.Client;
using Ip.Client.Services;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

// プラントは接続 1 本につき 1 台。アプリ全体で共有する。
builder.Services.AddSingleton<PlantRunner>();

await builder.Build().RunAsync();
