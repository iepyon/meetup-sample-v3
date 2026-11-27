# 残項目リスト（Rust 移行後）

2026-11-27 に TS から Rust（axum + sqlx + Leptos CSR）へ全面移行した時点で残っている項目。経緯と判断理由は [arch-20261127-rewrite-in-rust.md](decision-logs/arch-20261127-rewrite-in-rust.md) を参照。

移行時点の検証済み: fmt / clippy（native・wasm32）、`cargo test`（backend 単体 33 + 統合 47、frontend 10）、Playwright e2e 11 件。

完了した項目は `[x]` にして残す。

## 1. 未検証（実行していない）

- [ ] Docker イメージ / docker-compose / `d` スクリプトの動作確認（`d build && d up && d install && d review`）
- [ ] GitHub Actions CI の初回実行確認（quality / e2e ジョブ）
- [ ] `.env.example` に旧 Prisma / Vite 系の変数が残っていないか確認（AI は権限設定で読めないため人が確認）

## 2. 未対応（見送ったもの）

### セキュリティ・サプライチェーン
- [ ] Dockerfile の `rustup` が `curl | sh`（チェックサム未検証）。固定 URL + sha256 検証と `rust-toolchain.toml` でのバージョン固定
- [ ] GitHub Actions のサードパーティ Action（`dtolnay/rust-toolchain`、`Swatinem/rust-cache`）がタグ指定。commit SHA に固定
- [ ] CI の e2e で `cargo install trunk` が毎回約 4 分。ビルド済みバイナリ取得に変更して無料枠を節約
- [ ] セキュリティヘッダ（CSP / `X-Content-Type-Options` / `X-Frame-Options` / HSTS）。JWT は localStorage 保存のため `inner_html` を使わない運用が前提
- [ ] `JWT_SECRET` の最小長は警告のみ。必要なら起動拒否に変更（E2E / 開発用の短い値との兼ね合い）
- [ ] `reject` は ACTIVE な一般メンバーも削除できる（OWNER のみ保護）。PENDING のみ許可にするか決める
- [ ] `npm install -g @anthropic-ai/claude-code`（Dockerfile）のバージョン未固定

### 品質ゲート
- [ ] カバレッジ 80% ゲート（`cargo llvm-cov --fail-under-lines 80` を CI / `d review` に追加）
- [ ] 層依存・循環依存・複雑度の自動検査（旧 dependency-cruiser / madge 相当。`cargo-modules` 等）

### テスト
- [ ] E2E に PRIVATE コミュニティの承認フローと未ログイン時のリダイレクトを追加
- [ ] Leptos ページ/コンポーネントの単体テスト（現状は E2E 依存）
- [ ] 1 テストに複数検証が入っているもの（`api.rs` の参加/脱退、承認/拒否など）を「1 基準 1 テスト」に分割
- [ ] レート制限テストが実時間・固定回数（61 回など）に依存。時刻注入でウィンドウ復帰も検証
- [ ] `main.rs` の `JWT_SECRET` 判定を純粋関数に切り出してテスト

### コード
- [ ] `event::models::CreateEventInput` の `now` / `created_at` / `updated_at` の重複を整理
- [ ] `community/usecases.rs`（約 370 行）を command / query に分割（400 行制限に近い）
- [ ] `required_enum` 後の `parse(..).unwrap_or(既定値)` を型安全な `required_enum<T>` に置き換え
- [ ] `use super::errors::*` のグロブ import を named import に

## 3. ドキュメントの陳腐化

- [ ] `tutorial/1day-tutorial.md` が TS / ESLint / Vitest 前提のまま
- [ ] `docs/wiki/SCHEMA.md` の例示パス・リンクが旧構成のまま（人間所有のため未編集）
- [ ] wiki の `verified` SHA（`5891e84`）は旧リポジトリのもので、`.git` 作り直しにより無効。`/wiki sync` で再確認する

## 4. 運用・環境

- [ ] `git config user.name` が `“Eiji`（先頭が全角引用符）。`git config --global user.name "Eiji"` のあと `git commit --amend --reset-author`
- [ ] リモート未設定・push 未実施
- [ ] 旧 Prisma DB は読めない（データ移行手順なし）。必要なら移行スクリプトを作る
- [ ] `backend/prisma/seed.ts` を削除済み。開発用 seed が必要なら Rust 側に用意する

## 参考: 移行時に入れた意図的な挙動変更

詳細は decision-log の「意図した挙動の差異」「レビュー指摘への対応」を参照。

- コミュニティ一覧は未ログイン時 PUBLIC のみ
- PRIVATE コミュニティの詳細・イベント一覧は ACTIVE メンバーのみ（それ以外は 404）
- `reject` は OWNER 不可、パスワードは最大 72 バイト、メンバー ID 指定の脱退はコミュニティも照合
- ログイン失敗時はエラー表示して留まる
