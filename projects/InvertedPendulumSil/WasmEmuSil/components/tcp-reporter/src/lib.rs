//! tcp-reporter — capability の実証用部品。
//!
//! `everyMs` ごとに入力 `value` を `"<t_ns>,<value>\n"` の 1 行として `host:port` へ TCP で送り
//! (接続ごとに 1 行、送ったら閉じる)、`file` が指定されていれば同じ行をデータディレクトリ内のファイルに追記する。
//! WASI は `abi/wit/capabilities.wit` の `wasi-capabilities` world から必要なものだけ import する。
//! manifest には `net.tcpConnect: ["host:port"]` と (file を使うなら) `fs: [{path, mode: rw}]` の宣言が要る。
//! 宣言がなければエンジンは import を link せず、この部品は実体化できない (= 設計どおり)。
//!
//! 他の部品と同じく `wasm32-unknown-unknown` + `wasm-tools component new` でビルドする (D25)。
//! `wasm32-wasip2` の std を使わないのは、`wasi:cli/*` (stdout, exit, environment) が混ざるのを避けるため。

use ip_part_sdk::{num, scalar, PartDesc, PartImpl};

mod wasi {
    wit_bindgen::generate!({
        path: "../../abi/wit",
        world: "wasi-capabilities",
        generate_all,
    });
}

use wasi::wasi::filesystem::types::{DescriptorFlags, OpenFlags, PathFlags};
use wasi::wasi::sockets::network::{
    ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
};
use wasi::wasi::sockets::tcp::ShutdownType;
use wasi::wasi::sockets::{instance_network, ip_name_lookup, tcp_create_socket};

/// `last-error` 出力のコード。
const ERR_NONE: f64 = 0.0;
const ERR_ACCESS_DENIED: f64 = 1.0;
const ERR_CONNECT: f64 = 2.0;
const ERR_WRITE: f64 = 3.0;
const ERR_FILE: f64 = 4.0;
const ERR_RESOLVE: f64 = 5.0;

pub struct TcpReporter {
    host: String,
    port: u16,
    every_ns: i64,
    /// `<preopen>/<relative>` (preopen 名は最初の要素)
    file: Option<(String, String)>,
    value: f64,
    acc_ns: i64,
    t_ns: i64,
    sent: u32,
    errors: u32,
    last_error: f64,
    file_bytes: u64,
}

impl PartImpl for TcpReporter {
    fn describe() -> PartDesc {
        PartDesc {
            name: "tcp-reporter".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            inputs: vec![scalar("value", "", "送る値")],
            outputs: vec![
                scalar("sent", "count", "送信に成功した行数"),
                scalar("errors", "count", "失敗した回数 (接続拒否・解決失敗・書き込み失敗)"),
                scalar("last-error", "", "直近の失敗コード (0 なし, 1 許可外, 2 接続失敗, 3 送信失敗, 4 ファイル失敗, 5 名前解決失敗)"),
                scalar("file-bytes", "byte", "ファイルに追記した累計バイト数"),
            ],
            params: Vec::new(),
            // 通信の成否は外界に依存するが、部品自身は壁時計も乱数も使わない
            deterministic: true,
            doc: "入力値を周期的に TCP で送り、任意でデータディレクトリ内のファイルに追記する (capability 実証用)".into(),
        }
    }

    fn init(v: &serde_json::Value) -> Result<Self, String> {
        let host = v
            .get("host")
            .and_then(|h| h.as_str())
            .unwrap_or("127.0.0.1")
            .to_owned();
        let port = num(v, "port", 0.0);
        if !(1.0..=65535.0).contains(&port) {
            return Err("port は 1..65535".into());
        }
        let every_ms = num(v, "everyMs", 100.0);
        if !(every_ms >= 1.0) {
            return Err("everyMs は 1 以上".into());
        }
        let file = match v.get("file").and_then(|f| f.as_str()) {
            None | Some("") => None,
            Some(f) => {
                let (pre, rest) = f
                    .split_once('/')
                    .ok_or_else(|| "file は <preopen>/<path> の形式".to_owned())?;
                Some((pre.to_owned(), rest.to_owned()))
            }
        };
        Ok(Self {
            host,
            port: port as u16,
            every_ns: (every_ms * 1e6) as i64,
            file,
            value: 0.0,
            acc_ns: 0,
            t_ns: 0,
            sent: 0,
            errors: 0,
            last_error: ERR_NONE,
            file_bytes: 0,
        })
    }

    fn reset(&mut self) {
        self.value = 0.0;
        self.acc_ns = 0;
        self.t_ns = 0;
        self.sent = 0;
        self.errors = 0;
        self.last_error = ERR_NONE;
        self.file_bytes = 0;
    }

    fn set_inputs(&mut self, values: &[f64]) {
        if let Some(v) = values.first() {
            self.value = *v;
        }
    }

    fn step(&mut self, dt_ns: i64) -> Result<(), String> {
        self.t_ns += dt_ns;
        self.acc_ns += dt_ns;
        if self.acc_ns >= self.every_ns {
            self.acc_ns -= self.every_ns;
            let line = format!("{},{}\n", self.t_ns, self.value);
            match self.send_tcp(line.as_bytes()) {
                Ok(()) => self.sent += 1,
                Err(code) => self.fail(code),
            }
            if self.file.is_some() {
                match self.append_file(line.as_bytes()) {
                    Ok(n) => self.file_bytes += n,
                    Err(code) => self.fail(code),
                }
            }
        }
        Ok(())
    }

    fn get_outputs(&self, out: &mut Vec<f64>) {
        out.extend_from_slice(&[
            self.sent as f64,
            self.errors as f64,
            self.last_error,
            self.file_bytes as f64,
        ]);
    }
}

impl TcpReporter {
    fn fail(&mut self, code: f64) {
        self.errors += 1;
        self.last_error = code;
    }

    /// 宛先アドレスの解決。IP リテラルはそのまま、ホスト名は wasi:sockets/ip-name-lookup (許可ホスト限定) で引く。
    fn resolve(&self, net: &wasi::wasi::sockets::network::Network) -> Result<IpAddress, f64> {
        if let Ok(v4) = self.host.parse::<std::net::Ipv4Addr>() {
            let o = v4.octets();
            return Ok(IpAddress::Ipv4((o[0], o[1], o[2], o[3])));
        }
        if let Ok(v6) = self.host.parse::<std::net::Ipv6Addr>() {
            let s = v6.segments();
            return Ok(IpAddress::Ipv6((
                s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
            )));
        }
        let stream = ip_name_lookup::resolve_addresses(net, &self.host).map_err(|e| match e {
            ErrorCode::AccessDenied => ERR_ACCESS_DENIED,
            _ => ERR_RESOLVE,
        })?;
        loop {
            match stream.resolve_next_address() {
                Ok(Some(a)) => return Ok(a),
                Ok(None) => return Err(ERR_RESOLVE),
                Err(ErrorCode::WouldBlock) => stream.subscribe().block(),
                Err(ErrorCode::AccessDenied) => return Err(ERR_ACCESS_DENIED),
                Err(_) => return Err(ERR_RESOLVE),
            }
        }
    }

    fn send_tcp(&self, payload: &[u8]) -> Result<(), f64> {
        let net = instance_network::instance_network();
        let addr = self.resolve(&net)?;
        let (family, remote) = match addr {
            IpAddress::Ipv4(a) => (
                IpAddressFamily::Ipv4,
                IpSocketAddress::Ipv4(Ipv4SocketAddress {
                    port: self.port,
                    address: a,
                }),
            ),
            IpAddress::Ipv6(a) => (
                IpAddressFamily::Ipv6,
                IpSocketAddress::Ipv6(Ipv6SocketAddress {
                    port: self.port,
                    flow_info: 0,
                    scope_id: 0,
                    address: a,
                }),
            ),
        };
        let sock = tcp_create_socket::create_tcp_socket(family).map_err(|_| ERR_CONNECT)?;
        sock.start_connect(&net, remote).map_err(|e| match e {
            ErrorCode::AccessDenied => ERR_ACCESS_DENIED,
            _ => ERR_CONNECT,
        })?;
        let (_input, output) = loop {
            match sock.finish_connect() {
                Ok(streams) => break streams,
                Err(ErrorCode::WouldBlock) => sock.subscribe().block(),
                Err(ErrorCode::AccessDenied) => return Err(ERR_ACCESS_DENIED),
                Err(_) => return Err(ERR_CONNECT),
            }
        };
        output
            .blocking_write_and_flush(payload)
            .map_err(|_| ERR_WRITE)?;
        let _ = sock.shutdown(ShutdownType::Both);
        Ok(())
    }

    /// preopen (manifest の fs.path と同名) の配下に追記する。
    fn append_file(&self, payload: &[u8]) -> Result<u64, f64> {
        let (pre, rel) = self.file.as_ref().ok_or(ERR_FILE)?;
        let dir = wasi::wasi::filesystem::preopens::get_directories()
            .into_iter()
            .find(|(_, name)| name == pre)
            .map(|(d, _)| d)
            .ok_or(ERR_FILE)?;
        let fd = dir
            .open_at(
                PathFlags::empty(),
                rel,
                OpenFlags::CREATE,
                DescriptorFlags::WRITE,
            )
            .map_err(|_| ERR_FILE)?;
        let stream = fd.append_via_stream().map_err(|_| ERR_FILE)?;
        stream
            .blocking_write_and_flush(payload)
            .map_err(|_| ERR_FILE)?;
        Ok(payload.len() as u64)
    }
}

ip_part_sdk::part_entry!(TcpReporter);
