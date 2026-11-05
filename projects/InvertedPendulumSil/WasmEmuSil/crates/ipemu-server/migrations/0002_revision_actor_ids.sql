-- 編集者・承認者の分離をユーザ id で判定する (D41、監査 A1)。
-- created_by / updated_by は表示用のユーザ名として残し、判定には id を使う。
-- 既存の行は NULL のまま (承認時はユーザ名で比較するフォールバック)。
ALTER TABLE manifest_revisions ADD COLUMN created_by_id TEXT;
ALTER TABLE manifest_revisions ADD COLUMN updated_by_id TEXT;
