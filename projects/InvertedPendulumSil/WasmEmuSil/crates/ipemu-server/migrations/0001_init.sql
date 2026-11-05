-- ipemu-server 初期スキーマ (SQLite)。時刻はすべて RFC 3339 (UTC) の文字列。
-- JSON 列は TEXT に JSON をそのまま入れる (SQLite の json1 で読める)。

CREATE TABLE users (
    id                   TEXT PRIMARY KEY,
    username             TEXT NOT NULL UNIQUE,
    display_name         TEXT NOT NULL DEFAULT '',
    password_hash        TEXT NOT NULL,
    roles                TEXT NOT NULL DEFAULT '["user"]',   -- JSON 配列: user / viewer / editor / approver / admin
    groups_json          TEXT NOT NULL DEFAULT '[]',         -- JSON 配列: 公開範囲の照合に使うグループ名
    must_change_password INTEGER NOT NULL DEFAULT 0,
    disabled             INTEGER NOT NULL DEFAULT 0,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

-- 利用者セッション。login = ポータルのログイン、launch = 単回チケットから得た短命トークン
CREATE TABLE sessions (
    token_hash   TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    expires_at   TEXT NOT NULL,
    last_used_at TEXT
);
CREATE INDEX sessions_user ON sessions(user_id);

-- headless / CI 用のクライアント API キー
CREATE TABLE api_keys (
    id                TEXT PRIMARY KEY,
    name              TEXT NOT NULL,
    key_hash          TEXT NOT NULL UNIQUE,
    prefix            TEXT NOT NULL,
    allowed_manifests TEXT NOT NULL DEFAULT '["*"]',   -- JSON 配列: manifest id。"*" で全公開構成
    created_by        TEXT NOT NULL,
    created_at        TEXT NOT NULL,
    revoked_at        TEXT,
    last_used_at      TEXT
);

-- 登録済みアーティファクト (部品 / FW / UI)。本体は store/artifacts/sha256/ab/cd/<hex>
CREATE TABLE artifacts (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    version       TEXT NOT NULL,
    kind          TEXT NOT NULL,                 -- part / firmware / ui
    sha256        TEXT NOT NULL,
    size          INTEGER NOT NULL,
    content_type  TEXT NOT NULL,
    world         TEXT NOT NULL DEFAULT '',       -- 実装する WIT ワールド (part / firmware)
    describe_json TEXT,                           -- part の describe() (JSON)
    wasi_imports  TEXT NOT NULL DEFAULT '[]',     -- import する wasi:* インターフェース名 (JSON 配列)
    imports       TEXT NOT NULL DEFAULT '[]',     -- import 全件 (JSON 配列)
    uploaded_by   TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    UNIQUE(name, version)
);
CREATE INDEX artifacts_sha ON artifacts(sha256);

-- 構成 (id 単位)。公開範囲と既定 revision は本文ではなくここに持つ (署名のやり直しを避ける)
CREATE TABLE manifests (
    id                TEXT PRIMARY KEY,
    name              TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    visibility        TEXT NOT NULL DEFAULT 'all',   -- all / groups
    visibility_groups TEXT NOT NULL DEFAULT '[]',    -- JSON 配列
    default_revision  INTEGER,
    created_by        TEXT NOT NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

-- revision。state: draft → approved → published
CREATE TABLE manifest_revisions (
    manifest_id       TEXT NOT NULL REFERENCES manifests(id) ON DELETE CASCADE,
    revision          INTEGER NOT NULL,
    state             TEXT NOT NULL DEFAULT 'draft',
    body              TEXT NOT NULL,                 -- approval / signature を除いた本文 (JSON)
    body_sha256       TEXT NOT NULL,
    validation        TEXT,                          -- 最後の検証結果 (JSON)
    created_by        TEXT NOT NULL,                 -- ユーザ名 (approved_by / published_by と同じ)
    updated_by        TEXT NOT NULL,                 -- 最終編集者のユーザ名。承認者と同一なら承認を拒否する
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    approved_by       TEXT,
    approved_at       TEXT,
    approval_reason   TEXT,
    capabilities_hash TEXT,
    published_by      TEXT,
    published_at      TEXT,
    signed_body       TEXT,                          -- 公開時の署名付き全文 (JSON)
    key_id            TEXT,
    PRIMARY KEY (manifest_id, revision)
);

-- 外部ブラウザ (ipemu://) 用の単回チケット
CREATE TABLE launch_tickets (
    ticket_hash TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    manifest_id TEXT NOT NULL,
    revision    INTEGER,
    created_at  TEXT NOT NULL,
    expires_at  TEXT NOT NULL,
    used_at     TEXT
);

-- 起動履歴 (manifest 取得時に作り、telemetry で終わりを埋める)
CREATE TABLE launches (
    id              TEXT PRIMARY KEY,
    manifest_id     TEXT NOT NULL,
    revision        INTEGER NOT NULL,
    principal_kind  TEXT NOT NULL,   -- user / api_key
    principal_id    TEXT NOT NULL,
    principal_name  TEXT NOT NULL,
    credential_kind TEXT NOT NULL,   -- session / ticket / api_key
    client_info     TEXT,            -- JSON (engineVersion, os, arch, ...)
    started_at      TEXT NOT NULL,
    ended_at        TEXT,
    outcome         TEXT,            -- running / stopped / fault / ...
    detail          TEXT             -- JSON (最後の telemetry)
);
CREATE INDEX launches_principal ON launches(principal_kind, principal_id, started_at);
CREATE INDEX launches_manifest ON launches(manifest_id, started_at);

CREATE TABLE telemetry (
    id             TEXT PRIMARY KEY,
    launch_id      TEXT,
    principal_kind TEXT NOT NULL,
    principal_id   TEXT NOT NULL,
    manifest_id    TEXT NOT NULL,
    revision       INTEGER NOT NULL,
    event          TEXT NOT NULL,    -- started / stopped / fault / ...
    detail         TEXT,             -- JSON
    received_at    TEXT NOT NULL
);
CREATE INDEX telemetry_launch ON telemetry(launch_id);

-- hello してきたエンジン
CREATE TABLE clients (
    id             TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL,    -- エンジンが生成する識別子 (無ければ principal+os+arch から導出)
    principal_kind TEXT NOT NULL,
    principal_id   TEXT NOT NULL,
    principal_name TEXT NOT NULL,
    engine_version TEXT NOT NULL,
    wit_versions   TEXT NOT NULL DEFAULT '[]',
    os             TEXT NOT NULL,
    arch           TEXT NOT NULL,
    first_seen     TEXT NOT NULL,
    last_seen      TEXT NOT NULL,
    hello_count    INTEGER NOT NULL DEFAULT 1,
    UNIQUE(client_id, principal_kind, principal_id)
);

-- 監査ログ (すべての変更操作)
CREATE TABLE audit (
    id         TEXT PRIMARY KEY,
    at         TEXT NOT NULL,
    actor_id   TEXT NOT NULL,
    actor_name TEXT NOT NULL,
    action     TEXT NOT NULL,
    target     TEXT NOT NULL DEFAULT '',
    detail     TEXT,
    ip         TEXT
);
CREATE INDEX audit_at ON audit(at);
