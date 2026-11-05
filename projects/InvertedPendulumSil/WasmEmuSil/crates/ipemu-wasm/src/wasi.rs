//! capability の実施: 宣言・承認・同意済みの capability だけを WASI preview 2 として Linker に link する。
//!
//! 方針 (07-client-app.md「capability の実施」):
//! - **宣言のないものは import 自体を link しない。** 実体化が `unresolved import` で失敗し、承認なしに呼ばれる経路がない。
//! - `net` は `wasi:sockets/*` を link し、宛先は [`NetAllow`] (許可リスト) で `socket_addr_check` する。
//!   名前解決は自前の `wasi:sockets/ip-name-lookup` 実装で許可ホストに限定し、解決で得た IP だけを接続可にする。
//!   解決は別スレッドで行い、期限 ([`DnsConfig::timeout`]) を過ぎたら待たずに返す (監査 A7、D58)。
//! - `fs` はデータディレクトリ配下の宣言パスだけを `preopened_dir` で与える (ro / rw)。
//! - `clock` / `random` は link すると決定性の保証外 (`deterministic = false`)。
//! - `wasi:io/*` は sockets / filesystem / monotonic-clock の土台なので、いずれかの capability があるときだけ link する。
//!   `wasi:cli/*` (stdout 等) は与えない。

use anyhow::{bail, Context};
use ipemu_core::capability::{Capabilities, FsMode, HostPort};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use wasmtime::component::{HasData, Linker, Resource, ResourceTable};
use wasmtime_wasi::p2::bindings::sockets::network::{ErrorCode, IpAddress, Network};
use wasmtime_wasi::p2::{subscribe, DynPollable, Pollable};
use wasmtime_wasi::sockets::SocketAddrUse;
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// 実施に必要な形に解決した capability (fs はホスト側の実パスまで決めてある)。
#[derive(Debug, Clone, Default)]
pub struct WasiGrant {
    pub tcp_connect: Vec<HostPort>,
    pub tcp_listen: Vec<HostPort>,
    pub udp: Vec<HostPort>,
    pub fs: Vec<FsMount>,
    pub clock: bool,
    pub random: bool,
    pub host_ext: Vec<String>,
    /// deny-all 形: 宣言と同じインターフェース集合を link する (実体化と describe は通る) が、
    /// 許可リストは空・preopen なしで、接続・名前解決・ファイル操作はすべて拒否される。
    /// `validate` / `inspect` のように「実行しない」文脈で使う (D34)。
    pub deny_all: bool,
}

/// preopen 1 件。
#[derive(Debug, Clone)]
pub struct FsMount {
    /// ホスト側の実パス (データディレクトリ配下であることを解決時に確認済み)
    pub host_path: PathBuf,
    /// ゲストから見える名前 (manifest の `fs[].path` そのまま)
    pub guest_path: String,
    pub mode: FsMode,
}

impl WasiGrant {
    /// 宣言からの変換。`fs` は `<data_root>/<path>` に解決し、無ければ作る。
    /// 閉じ込めは文字列の規則 ([`ipemu_core::capability::FsGrant::is_confined_path`]) と、
    /// 実体化後の canonical パスが `data_root` 配下にあること (シンボリックリンク経由の脱出) の両方で確認する。
    pub fn from_capabilities(caps: &Capabilities, data_root: &Path) -> anyhow::Result<Self> {
        let mut g = WasiGrant {
            clock: caps.clock,
            random: caps.random,
            host_ext: caps.host_ext.clone(),
            ..Default::default()
        };
        if let Some(n) = &caps.net {
            g.tcp_connect = n.tcp_connect.clone();
            g.tcp_listen = n.tcp_listen.clone();
            g.udp = n.udp.clone();
        }
        if !caps.fs.is_empty() {
            std::fs::create_dir_all(data_root).with_context(|| {
                format!("データディレクトリ {} を作れません", data_root.display())
            })?;
            let root = data_root.canonicalize().with_context(|| {
                format!(
                    "データディレクトリ {} を解決できません",
                    data_root.display()
                )
            })?;
            for f in &caps.fs {
                if !ipemu_core::capability::FsGrant::is_confined_path(&f.path) {
                    bail!(
                        "fs.path `{}` はデータディレクトリ配下の相対パスではありません",
                        f.path
                    );
                }
                let host = root.join(&f.path);
                std::fs::create_dir_all(&host)
                    .with_context(|| format!("{} を作れません", host.display()))?;
                let canon = host.canonicalize()?;
                if !canon.starts_with(&root) {
                    bail!(
                        "fs.path `{}` はデータディレクトリの外 ({}) を指しています",
                        f.path,
                        canon.display()
                    );
                }
                g.fs.push(FsMount {
                    host_path: canon,
                    guest_path: f.path.clone(),
                    mode: f.mode,
                });
            }
        }
        Ok(g)
    }

    /// 宣言に対応するインターフェースは link するが何も許可しない形 (D34)。
    /// fs はディレクトリを作らず preopen もしないので、データディレクトリに触れない。
    pub fn deny_all_for(caps: &Capabilities) -> Self {
        let mut g = WasiGrant {
            clock: caps.clock,
            random: caps.random,
            host_ext: caps.host_ext.clone(),
            deny_all: true,
            ..Default::default()
        };
        if let Some(n) = &caps.net {
            g.tcp_connect = n.tcp_connect.clone();
            g.tcp_listen = n.tcp_listen.clone();
            g.udp = n.udp.clone();
        }
        g.fs = caps
            .fs
            .iter()
            .map(|f| FsMount {
                host_path: PathBuf::new(),
                guest_path: f.path.clone(),
                mode: f.mode,
            })
            .collect();
        g
    }

    pub fn uses_tcp(&self) -> bool {
        !self.tcp_connect.is_empty() || !self.tcp_listen.is_empty()
    }
    pub fn uses_udp(&self) -> bool {
        !self.udp.is_empty()
    }
    pub fn uses_net(&self) -> bool {
        self.uses_tcp() || self.uses_udp()
    }
    pub fn uses_fs(&self) -> bool {
        !self.fs.is_empty()
    }
    /// `wasi:io/*` を link する必要があるか。
    pub fn uses_wasi(&self) -> bool {
        self.uses_net() || self.uses_fs() || self.clock
    }
    /// 決定性の保証外になるか (V10 と同じ判定)。
    pub fn nondeterministic(&self) -> bool {
        self.clock || self.random
    }
    pub fn is_empty(&self) -> bool {
        !self.uses_wasi() && !self.random && self.host_ext.is_empty()
    }
}

/// 名前解決の実体。既定は OS ([`SystemResolver`])。テストでは止まる DNS などに差し替える。
pub trait Resolver: Send + Sync {
    fn lookup(&self, name: &str) -> std::io::Result<Vec<IpAddr>>;
}

/// OS の名前解決 (`getaddrinfo`)。呼び出したスレッドを DNS の応答まで止める。
#[derive(Debug, Default)]
pub struct SystemResolver;

impl Resolver for SystemResolver {
    fn lookup(&self, name: &str) -> std::io::Result<Vec<IpAddr>> {
        Ok((name, 0u16).to_socket_addrs()?.map(|a| a.ip()).collect())
    }
}

/// 名前解決の期限とキャッシュ (D58)。
#[derive(Clone)]
pub struct DnsConfig {
    /// 1 回の `resolve-addresses` が待つ上限 (壁時計)。過ぎたら `temporary-resolver-failure`
    pub timeout: Duration,
    /// 解決結果を再利用する期間。失敗は覚えない
    pub cache_ttl: Duration,
    pub resolver: Arc<dyn Resolver>,
}

impl DnsConfig {
    /// 既定の期限。epoch ウォッチドッグの既定予算 (5 s) より短くし、1 回の待ちで呼び出しが trap しないようにする
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);
    pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(60);
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            timeout: Self::DEFAULT_TIMEOUT,
            cache_ttl: Self::DEFAULT_CACHE_TTL,
            resolver: Arc::new(SystemResolver),
        }
    }
}

impl std::fmt::Debug for DnsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DnsConfig")
            .field("timeout", &self.timeout)
            .field("cache_ttl", &self.cache_ttl)
            .finish_non_exhaustive()
    }
}

/// キャッシュに置く名前の上限。
const DNS_CACHE_MAX: usize = 32;
/// 同時に走らせる解決スレッドの上限 (`*.example.com` の許可で名前を変えながら呼ばれても増え続けない)。
const DNS_INFLIGHT_MAX: usize = 4;

type Lookup = Result<Vec<IpAddr>, ErrorCode>;

/// 解決スレッド 1 本の結果の受け渡し口。
#[derive(Debug, Default)]
struct PendingLookup {
    result: Mutex<Option<Lookup>>,
    done: Condvar,
}

#[derive(Debug, Default)]
struct DnsState {
    /// 名前 (小文字) → (解決した時刻, IP)
    cache: HashMap<String, (Instant, Vec<IpAddr>)>,
    /// 走っている (または結果をまだ回収していない) 解決。同じ名前は 1 本にまとめる
    inflight: HashMap<String, Arc<PendingLookup>>,
}

impl DnsState {
    fn remember(&mut self, key: String, ips: Vec<IpAddr>) {
        if self.cache.len() >= DNS_CACHE_MAX && !self.cache.contains_key(&key) {
            if let Some(oldest) = self
                .cache
                .iter()
                .min_by_key(|(_, (at, _))| *at)
                .map(|(k, _)| k.clone())
            {
                self.cache.remove(&oldest);
            }
        }
        self.cache.insert(key, (Instant::now(), ips));
    }

    /// 期限切れの後に終わった解決を回収する (成功はキャッシュへ)。
    fn reap(&mut self) {
        let finished: Vec<(String, Lookup)> = self
            .inflight
            .iter()
            .filter_map(|(k, p)| {
                let r = p.result.lock().unwrap_or_else(|e| e.into_inner());
                r.clone().map(|r| (k.clone(), r))
            })
            .collect();
        for (k, r) in finished {
            self.inflight.remove(&k);
            if let Ok(ips) = r {
                self.remember(k, ips);
            }
        }
    }
}

/// ネットワークの許可リスト。`socket_addr_check` と名前解決が共有する。
#[derive(Debug, Default)]
pub struct NetAllow {
    tcp_connect: Vec<HostPort>,
    tcp_listen: Vec<HostPort>,
    udp: Vec<HostPort>,
    /// 許可ホスト名の解決で得た (IP, 許可エントリ)。この IP への接続はそのエントリのポートに限り許す
    resolved: Mutex<Vec<(IpAddr, HostPort)>>,
    /// 拒否した宛先 (診断用)
    denied: Mutex<Vec<String>>,
    dns_config: DnsConfig,
    dns: Mutex<DnsState>,
}

impl NetAllow {
    fn new(g: &WasiGrant) -> Self {
        Self::with_dns(g, DnsConfig::default())
    }

    /// 名前解決の期限・キャッシュ・実体を指定して作る。
    pub fn with_dns(g: &WasiGrant, dns_config: DnsConfig) -> Self {
        // deny-all 形では許可リストを空にする (インターフェースは link されるが全宛先を拒否)
        let pick = |v: &Vec<HostPort>| if g.deny_all { Vec::new() } else { v.clone() };
        Self {
            tcp_connect: pick(&g.tcp_connect),
            tcp_listen: pick(&g.tcp_listen),
            udp: pick(&g.udp),
            resolved: Mutex::new(Vec::new()),
            denied: Mutex::new(Vec::new()),
            dns_config,
            dns: Mutex::new(DnsState::default()),
        }
    }

    /// 許可済みの名前を、キャッシュ → 解決スレッド (期限付き) の順で引く。
    /// 期限を過ぎてもスレッドは止められないので走らせたままにし、結果は次の呼び出しで回収する。
    fn lookup(&self, name: &str) -> Lookup {
        let key = name.to_ascii_lowercase();
        let pending = {
            let mut d = self.dns.lock().unwrap_or_else(|e| e.into_inner());
            d.reap();
            if let Some((at, ips)) = d.cache.get(&key) {
                if at.elapsed() < self.dns_config.cache_ttl {
                    return Ok(ips.clone());
                }
            }
            match d.inflight.get(&key) {
                Some(p) => p.clone(),
                None => {
                    if d.inflight.len() >= DNS_INFLIGHT_MAX {
                        return Err(ErrorCode::TemporaryResolverFailure);
                    }
                    let p = Arc::new(PendingLookup::default());
                    let (p2, resolver, n) =
                        (p.clone(), self.dns_config.resolver.clone(), name.to_owned());
                    std::thread::Builder::new()
                        .name("ipemu-dns".into())
                        .spawn(move || {
                            // 解決順は OS のまま (先頭を使う部品が多い)。重複だけ除く
                            let r = resolver
                                .lookup(&n)
                                .map(|ips| {
                                    let mut seen = HashSet::new();
                                    ips.into_iter()
                                        .map(|ip| ip.to_canonical())
                                        .filter(|ip| seen.insert(*ip))
                                        .collect()
                                })
                                .map_err(|_| ErrorCode::NameUnresolvable);
                            *p2.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                            p2.done.notify_all();
                        })
                        .map_err(|_| ErrorCode::TemporaryResolverFailure)?;
                    d.inflight.insert(key.clone(), p.clone());
                    p
                }
            }
        };
        let deadline = Instant::now() + self.dns_config.timeout;
        let mut r = pending.result.lock().unwrap_or_else(|e| e.into_inner());
        while r.is_none() {
            let now = Instant::now();
            if now >= deadline {
                return Err(ErrorCode::TemporaryResolverFailure);
            }
            r = pending
                .done
                .wait_timeout(r, deadline - now)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        let result = r.clone().expect("待ち合わせで結果を確認済み");
        drop(r);
        let mut d = self.dns.lock().unwrap_or_else(|e| e.into_inner());
        if d.inflight
            .get(&key)
            .is_some_and(|p| Arc::ptr_eq(p, &pending))
        {
            d.inflight.remove(&key);
        }
        if let Ok(ips) = &result {
            d.remember(key, ips.clone());
        }
        result
    }

    fn permits(&self, list: &[HostPort], addr: SocketAddr) -> bool {
        if list.iter().any(|h| h.matches_addr(&addr)) {
            return true;
        }
        let resolved = self.resolved.lock().unwrap_or_else(|e| e.into_inner());
        list.iter().any(|h| {
            h.is_name()
                && h.port == addr.port()
                && resolved
                    .iter()
                    .any(|(ip, entry)| entry == h && ip.to_canonical() == addr.ip().to_canonical())
        })
    }

    /// Wasmtime からの問い合わせ。暗黙の bind (未指定アドレス:0) は接続 / 送信の前提として許す。
    pub fn check(&self, addr: SocketAddr, use_: SocketAddrUse) -> bool {
        let implicit = addr.ip().is_unspecified() && addr.port() == 0;
        let ok = match use_ {
            SocketAddrUse::TcpConnect => self.permits(&self.tcp_connect, addr),
            SocketAddrUse::TcpBind => {
                (implicit && !self.tcp_connect.is_empty()) || self.permits(&self.tcp_listen, addr)
            }
            SocketAddrUse::TcpListen => self.permits(&self.tcp_listen, addr),
            // accept の相手は宣言で縛れない (listen を許した時点で受け入れる)
            SocketAddrUse::TcpAccept => !self.tcp_listen.is_empty(),
            SocketAddrUse::UdpBind => {
                (implicit && !self.udp.is_empty()) || self.permits(&self.udp, addr)
            }
            SocketAddrUse::UdpSend | SocketAddrUse::UdpReceive => self.permits(&self.udp, addr),
        };
        if !ok {
            let mut d = self.denied.lock().unwrap_or_else(|e| e.into_inner());
            if d.len() < 64 {
                d.push(format!("{use_:?} {addr}"));
            }
        }
        ok
    }

    /// 許可ホスト名だけを解決する。IP リテラルは宣言にあるものだけ通す。
    pub fn resolve(&self, name: &str) -> Result<Vec<IpAddr>, ErrorCode> {
        let entries: Vec<&HostPort> = self
            .tcp_connect
            .iter()
            .chain(&self.tcp_listen)
            .chain(&self.udp)
            .filter(|h| h.matches_name(name))
            .collect();
        if entries.is_empty() {
            let mut d = self.denied.lock().unwrap_or_else(|e| e.into_inner());
            if d.len() < 64 {
                d.push(format!("resolve {name}"));
            }
            return Err(ErrorCode::AccessDenied);
        }
        if let Ok(ip) = name.parse::<IpAddr>() {
            return Ok(vec![ip]);
        }
        let ips = self.lookup(name)?;
        let mut resolved = self.resolved.lock().unwrap_or_else(|e| e.into_inner());
        for ip in &ips {
            for e in &entries {
                if !resolved.iter().any(|(i, h)| i == ip && h == *e) {
                    resolved.push((*ip, (*e).clone()));
                }
            }
        }
        Ok(ips)
    }

    /// 拒否した宛先の一覧を取り出す (ログ用)。
    pub fn take_denied(&self) -> Vec<String> {
        std::mem::take(&mut *self.denied.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

/// `hostExt:serial-port` のスタブ (M5)。実機のシリアル機器への橋渡しは M6 以降。
#[derive(Debug, Default)]
pub struct SerialPortStub {
    pub calls: u32,
}

/// Store データに同居させる WASI の状態。部品 (`PartHost`) とファームウェア (`FwHost`) が持つ。
pub struct WasiState {
    pub ctx: WasiCtx,
    pub table: ResourceTable,
    pub allow: Arc<NetAllow>,
    pub serial: SerialPortStub,
    pub grant: WasiGrant,
}

impl WasiState {
    /// 付与内容から WasiCtx を組む。stdio・環境変数・引数は与えない。
    pub fn new(grant: WasiGrant) -> anyhow::Result<Self> {
        let allow = Arc::new(NetAllow::new(&grant));
        let mut b = WasiCtxBuilder::new();
        // ファイル操作は呼び出しスレッドで同期実行 (スケジューラは単一スレッド)
        b.allow_blocking_current_thread(true);
        b.allow_tcp(grant.uses_tcp());
        b.allow_udp(grant.uses_udp());
        // 名前解決は wasmtime-wasi の実装ではなく、許可ホスト限定の自前実装 (link_name_lookup) を使う
        b.allow_ip_name_lookup(false);
        if grant.uses_net() {
            let a = allow.clone();
            b.socket_addr_check(move |addr, use_| {
                let ok = a.check(addr, use_);
                Box::pin(async move { ok })
            });
        }
        for m in grant.fs.iter().filter(|_| !grant.deny_all) {
            let perms = match m.mode {
                FsMode::Ro => FsPerms::ReadOnly,
                FsMode::Rw => FsPerms::ReadWrite,
            };
            b.preopened_dir(&m.host_path, &m.guest_path, perms)
                .map_err(|e| {
                    anyhow::anyhow!("preopen {} → {}: {e}", m.host_path.display(), m.guest_path)
                })?;
        }
        Ok(Self {
            ctx: b.build(),
            table: ResourceTable::new(),
            allow,
            serial: SerialPortStub::default(),
            grant,
        })
    }

    pub fn view(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

/// WASI を持てる Store データ。`WasiView` の実装は各ホスト型が `wasi().view()` で委譲する。
pub trait WasiHost: WasiView + 'static {
    fn wasi(&mut self) -> &mut WasiState;
}

struct HasIo;
impl HasData for HasIo {
    type Data<'a> = &'a mut ResourceTable;
}

/// 付与された capability に対応するインターフェースだけを link する。
pub fn add_to_linker<T: WasiHost>(linker: &mut Linker<T>, grant: &WasiGrant) -> anyhow::Result<()> {
    use wasmtime_wasi::clocks::WasiClocks;
    use wasmtime_wasi::filesystem::{WasiFilesystem, WasiFilesystemView};
    use wasmtime_wasi::p2::bindings::{self, sync};
    use wasmtime_wasi::random::WasiRandom;
    use wasmtime_wasi::sockets::{WasiSockets, WasiSocketsView};
    let l = linker;

    if grant.uses_wasi() {
        sync::io::error::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
        sync::io::poll::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
        sync::io::streams::add_to_linker::<T, HasIo>(l, |t| t.ctx().table)?;
    }
    if grant.uses_net() {
        let opts = bindings::sockets::network::LinkOptions::default();
        bindings::sockets::network::add_to_linker::<T, WasiSockets>(l, &opts, T::sockets)?;
        bindings::sockets::instance_network::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
        if grant.uses_tcp() {
            sync::sockets::tcp::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
            bindings::sockets::tcp_create_socket::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
        }
        if grant.uses_udp() {
            sync::sockets::udp::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
            sync::sockets::udp_create_socket::add_to_linker::<T, WasiSockets>(l, T::sockets)?;
        }
        name_lookup::add_to_linker(l)?;
    }
    if grant.uses_fs() {
        sync::filesystem::types::add_to_linker::<T, WasiFilesystem>(l, T::filesystem)?;
        bindings::filesystem::preopens::add_to_linker::<T, WasiFilesystem>(l, T::filesystem)?;
    }
    if grant.clock {
        use wasmtime_wasi::clocks::WasiClocksView;
        bindings::clocks::wall_clock::add_to_linker::<T, WasiClocks>(l, T::clocks)?;
        bindings::clocks::monotonic_clock::add_to_linker::<T, WasiClocks>(l, T::clocks)?;
    }
    if grant.random {
        use wasmtime_wasi::random::WasiRandomView;
        bindings::random::random::add_to_linker::<T, WasiRandom>(l, T::random)?;
        bindings::random::insecure::add_to_linker::<T, WasiRandom>(l, T::random)?;
        bindings::random::insecure_seed::add_to_linker::<T, WasiRandom>(l, T::random)?;
    }
    for ext in &grant.host_ext {
        match ext.as_str() {
            "serial-port" => host_ext::link_serial_port(l)?,
            other => bail!("hostExt `{other}` はこのエンジンが提供しません"),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// wasi:sockets/ip-name-lookup の自前実装 (許可ホスト限定)
// ---------------------------------------------------------------------------
mod name_lookup {
    use super::*;

    mod bindings {
        wasmtime::component::bindgen!({
            path: "../../abi/wit",
            world: "name-lookup-host",
            imports: { default: trappable },
            with: {
                "wasi:sockets/network": wasmtime_wasi::p2::bindings::sockets::network,
                "wasi:io/poll": wasmtime_wasi::p2::bindings::sync::io::poll,
                "wasi:sockets/ip-name-lookup.resolve-address-stream": super::ResolvedAddresses,
            },
        });
    }
    use bindings::wasi::sockets::ip_name_lookup::{Host, HostResolveAddressStream};

    /// 解決済みアドレスの列。期限付きで待ってから返すので pollable は常に ready。
    pub struct ResolvedAddresses(std::vec::IntoIter<IpAddr>);

    #[wasmtime_wasi::async_trait]
    impl Pollable for ResolvedAddresses {
        async fn ready(&mut self) {}
    }

    pub struct View<'a> {
        table: &'a mut ResourceTable,
        allow: &'a NetAllow,
    }

    struct HasView;
    impl HasData for HasView {
        type Data<'a> = View<'a>;
    }

    impl Host for View<'_> {
        fn resolve_addresses(
            &mut self,
            network: Resource<Network>,
            name: String,
        ) -> wasmtime::Result<Result<Resource<ResolvedAddresses>, ErrorCode>> {
            // network リソースの有効性だけ確認する (wasmtime-wasi と同じ)
            self.table.get(&network)?;
            Ok(match self.allow.resolve(&name) {
                Ok(ips) => Ok(self.table.push(ResolvedAddresses(ips.into_iter()))?),
                Err(e) => Err(e),
            })
        }
    }

    impl HostResolveAddressStream for View<'_> {
        fn resolve_next_address(
            &mut self,
            this: Resource<ResolvedAddresses>,
        ) -> wasmtime::Result<Result<Option<IpAddress>, ErrorCode>> {
            let s = self.table.get_mut(&this)?;
            Ok(Ok(s.0.next().map(IpAddress::from)))
        }
        fn subscribe(
            &mut self,
            this: Resource<ResolvedAddresses>,
        ) -> wasmtime::Result<Resource<DynPollable>> {
            subscribe(self.table, this)
        }
        fn drop(&mut self, this: Resource<ResolvedAddresses>) -> wasmtime::Result<()> {
            self.table.delete(this)?;
            Ok(())
        }
    }

    pub fn add_to_linker<T: WasiHost>(l: &mut Linker<T>) -> anyhow::Result<()> {
        bindings::wasi::sockets::ip_name_lookup::add_to_linker::<T, HasView>(l, |t| {
            let s = t.wasi();
            View {
                table: &mut s.table,
                allow: &s.allow,
            }
        })?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// hostExt: ip:emu/serial-port (M5 ではスタブ)
// ---------------------------------------------------------------------------
mod host_ext {
    use super::*;

    mod bindings {
        wasmtime::component::bindgen!({
            path: "../../abi/wit",
            world: "host-ext",
        });
    }

    struct HasSerial;
    impl HasData for HasSerial {
        type Data<'a> = &'a mut SerialPortStub;
    }

    impl bindings::ip::emu::serial_port::Host for SerialPortStub {
        fn open(&mut self, path: String, _baud: u32) -> Result<u32, String> {
            self.calls += 1;
            Err(format!("serial-port `{path}`: このエンジンではまだ実機のシリアルポートに接続できません (M5 スタブ)"))
        }
        fn write(&mut self, _handle: u32, _data: Vec<u8>) -> Result<u32, String> {
            self.calls += 1;
            Err("serial-port: 未接続".into())
        }
        fn read(&mut self, _handle: u32, _max: u32) -> Result<Vec<u8>, String> {
            self.calls += 1;
            Err("serial-port: 未接続".into())
        }
        fn close(&mut self, _handle: u32) {
            self.calls += 1;
        }
    }

    pub fn link_serial_port<T: WasiHost>(l: &mut Linker<T>) -> anyhow::Result<()> {
        bindings::ip::emu::serial_port::add_to_linker::<T, HasSerial>(l, |t| &mut t.wasi().serial)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(connect: &[&str], listen: &[&str], udp: &[&str]) -> WasiGrant {
        let p = |v: &[&str]| v.iter().map(|s| HostPort::parse(s).unwrap()).collect();
        WasiGrant {
            tcp_connect: p(connect),
            tcp_listen: p(listen),
            udp: p(udp),
            ..Default::default()
        }
    }

    #[test]
    fn addr_check_follows_allow_lists() {
        let a = NetAllow::new(&grant(&["127.0.0.1:9000"], &["127.0.0.1:9100"], &[]));
        let ok: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let other: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        assert!(a.check(ok, SocketAddrUse::TcpConnect));
        assert!(!a.check(other, SocketAddrUse::TcpConnect));
        // connect のための暗黙 bind は許す。明示 bind は listen のリストに従う
        assert!(a.check("0.0.0.0:0".parse().unwrap(), SocketAddrUse::TcpBind));
        assert!(a.check("127.0.0.1:9100".parse().unwrap(), SocketAddrUse::TcpBind));
        assert!(!a.check("127.0.0.1:9000".parse().unwrap(), SocketAddrUse::TcpBind));
        assert!(a.check("127.0.0.1:9100".parse().unwrap(), SocketAddrUse::TcpListen));
        // udp は宣言なし
        assert!(!a.check("0.0.0.0:0".parse().unwrap(), SocketAddrUse::UdpBind));
        assert_eq!(a.take_denied().len(), 3);
    }

    #[test]
    fn name_resolution_is_limited_to_declared_hosts() {
        let a = NetAllow::new(&grant(
            &["localhost:9000", "*.example.invalid:80"],
            &[],
            &[],
        ));
        assert_eq!(
            a.resolve("evil.invalid").unwrap_err(),
            ErrorCode::AccessDenied
        );
        assert_eq!(a.resolve("127.0.0.1").unwrap_err(), ErrorCode::AccessDenied);
        let ips = a.resolve("localhost").unwrap();
        assert!(!ips.is_empty());
        // 解決で得た IP には、そのエントリのポートだけ接続できる
        let ip = ips[0];
        assert!(a.check(SocketAddr::new(ip, 9000), SocketAddrUse::TcpConnect));
        assert!(!a.check(SocketAddr::new(ip, 9001), SocketAddrUse::TcpConnect));
        // サブドメインの名前は許可対象 (解決自体は失敗してよい)
        assert_ne!(
            a.resolve("a.example.invalid").err(),
            Some(ErrorCode::AccessDenied)
        );
    }

    /// 呼ばれた回数を数え、`gate` が開くまで (または即座に) 192.0.2.7 を返す解決器。
    struct FakeResolver {
        calls: std::sync::atomic::AtomicU32,
        gate: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
    }
    impl FakeResolver {
        fn new(gate: Option<std::sync::mpsc::Receiver<()>>) -> Arc<Self> {
            Arc::new(Self {
                calls: Default::default(),
                gate: Mutex::new(gate),
            })
        }
        fn calls(&self) -> u32 {
            self.calls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }
    impl Resolver for FakeResolver {
        fn lookup(&self, _name: &str) -> std::io::Result<Vec<IpAddr>> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if let Some(rx) = self.gate.lock().unwrap().take() {
                let _ = rx.recv();
            }
            Ok(vec![
                "192.0.2.7".parse().unwrap(),
                "192.0.2.7".parse().unwrap(),
            ])
        }
    }

    fn dns(resolver: Arc<FakeResolver>, timeout_ms: u64) -> DnsConfig {
        DnsConfig {
            timeout: Duration::from_millis(timeout_ms),
            cache_ttl: Duration::from_secs(60),
            resolver,
        }
    }

    #[test]
    fn hanging_resolver_returns_within_deadline_and_result_is_reaped_later() {
        let (tx, rx) = std::sync::mpsc::channel();
        let r = FakeResolver::new(Some(rx));
        let a = NetAllow::with_dns(
            &grant(&["slow.example.test:80"], &[], &[]),
            dns(r.clone(), 150),
        );
        let t0 = Instant::now();
        assert_eq!(
            a.resolve("slow.example.test").unwrap_err(),
            ErrorCode::TemporaryResolverFailure
        );
        let waited = t0.elapsed();
        assert!(
            waited >= Duration::from_millis(150) && waited < Duration::from_secs(2),
            "期限 150ms で返っていない: {waited:?}"
        );
        // 期限切れの間は再試行しても新しいスレッドを立てず、同じ解決を待つ
        assert!(a.resolve("slow.example.test").is_err());
        assert_eq!(r.calls(), 1);
        // DNS が応答したら、次の呼び出しで結果を回収して使う (許可リストの規則は従来どおり)
        tx.send(()).unwrap();
        let t1 = Instant::now();
        let ips = loop {
            match a.resolve("slow.example.test") {
                Ok(ips) => break ips,
                Err(_) if t1.elapsed() < Duration::from_secs(5) => continue,
                Err(e) => panic!("解決できない: {e:?}"),
            }
        };
        assert_eq!(ips, vec!["192.0.2.7".parse::<IpAddr>().unwrap()]);
        assert_eq!(r.calls(), 1);
        assert!(a.check("192.0.2.7:80".parse().unwrap(), SocketAddrUse::TcpConnect));
        assert!(!a.check("192.0.2.7:81".parse().unwrap(), SocketAddrUse::TcpConnect));
    }

    #[test]
    fn resolution_is_cached_per_allow_list() {
        let r = FakeResolver::new(None);
        let a = NetAllow::with_dns(
            &grant(&["db.example.test:5432"], &[], &[]),
            dns(r.clone(), 1000),
        );
        let first = a.resolve("db.example.test").unwrap();
        let second = a.resolve("DB.example.test").unwrap();
        assert_eq!(first, second);
        assert_eq!(r.calls(), 1, "2 回目はキャッシュから返る");
        // 許可外の名前はキャッシュや解決器に届く前に拒否される
        assert_eq!(
            a.resolve("other.example.test").unwrap_err(),
            ErrorCode::AccessDenied
        );
        assert_eq!(r.calls(), 1);
        // 期限 0 のキャッシュなら毎回引き直す
        let r2 = FakeResolver::new(None);
        let mut cfg = dns(r2.clone(), 1000);
        cfg.cache_ttl = Duration::ZERO;
        let b = NetAllow::with_dns(&grant(&["db.example.test:5432"], &[], &[]), cfg);
        b.resolve("db.example.test").unwrap();
        b.resolve("db.example.test").unwrap();
        assert_eq!(r2.calls(), 2);
    }

    #[test]
    fn deny_all_links_same_set_but_permits_nothing() {
        let caps: Capabilities = serde_json::from_value(serde_json::json!({
            "net": { "tcpConnect": ["127.0.0.1:9000", "localhost:9000"] },
            "fs": [{ "path": "logs", "mode": "rw" }]
        }))
        .unwrap();
        let g = WasiGrant::deny_all_for(&caps);
        assert!(g.uses_tcp() && g.uses_fs() && g.deny_all);
        let a = NetAllow::new(&g);
        assert!(!a.check("127.0.0.1:9000".parse().unwrap(), SocketAddrUse::TcpConnect));
        assert!(!a.check("0.0.0.0:0".parse().unwrap(), SocketAddrUse::TcpBind));
        assert_eq!(a.resolve("localhost").unwrap_err(), ErrorCode::AccessDenied);
        // preopen は作られない (WasiState が組めることだけ確認)
        let st = WasiState::new(g).unwrap();
        assert!(st.grant.deny_all);
    }

    #[test]
    fn fs_mounts_are_confined_to_data_root() {
        let dir = std::env::temp_dir().join(format!("ipemu-wasi-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let caps: Capabilities = serde_json::from_value(serde_json::json!({
            "fs": [{ "path": "logs/run1", "mode": "rw" }]
        }))
        .unwrap();
        let g = WasiGrant::from_capabilities(&caps, &dir).unwrap();
        assert_eq!(g.fs.len(), 1);
        assert!(g.fs[0].host_path.starts_with(dir.canonicalize().unwrap()));
        assert!(g.fs[0].host_path.is_dir());
        let bad: Capabilities =
            serde_json::from_value(serde_json::json!({ "fs": [{ "path": "../x", "mode": "ro" }] }))
                .unwrap();
        assert!(WasiGrant::from_capabilities(&bad, &dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
