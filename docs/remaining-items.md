# 残項目リスト（Rust 移行後）

2026-11-27 に TS から Rust（axum + sqlx + Leptos CSR）へ全面移行した時点で残っている項目。経緯と判断理由は [arch-20261127-rewrite-in-rust.md](decision-logs/arch-20261127-rewrite-in-rust.md) を参照。

移行時点の検証済み: fmt / clippy（native・wasm32）、`cargo test`（backend 単体 33 + 統合 47、frontend 10）、Playwright e2e 11 件。

完了した項目は `[x]` にして残す。

## 1. 未検証（実行していない）

- [x] Docker イメージ / docker-compose / `d` スクリプトの動作確認（`d build && d up && d install && d review`）— 2026-11-27 に colima（aarch64）上で確認。当初 trunk が取得する wasm-bindgen が GLIBC 2.38 を要求してコンテナ内で動かず、既存の `frontend/dist` を配信したまま E2E が通っていた。Dockerfile で `wasm-bindgen-cli` をソースから導入して解消し、新しいビルドで `d review`・`d e2e`（13 件）の通過を確認
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
- [ ] E2E に未ログイン時のリダイレクトを追加（PRIVATE コミュニティの承認／拒否は 2026-11-27 に追加済み）
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

## 5. 移植監査で見つかったもの（2026-11-27）

仕様文書（wiki / issues / E2E）と Rust 実装を突き合わせた静的監査の結果。

### 不具合（コードで確認済み）
- [x] PRIVATE コミュニティの承認／拒否が UI から操作できない。メンバー一覧が ACTIVE のみを返すため PENDING の申請者が表示されず、承認ボタンは ACTIVE メンバーに出て押すと 422、拒否は追い出しになる — メンバー一覧は ACTIVE な OWNER / ADMIN にだけ PENDING も返し、承認／拒否ボタンは PENDING の行にだけ出すよう修正。E2E に承認・拒否を追加。これに伴い ACTIVE メンバーを UI から外す手段（旧「拒否」ボタン）は無くなった
- [x] E2E のイベント開始日時が `2026-12-01T19:00` 固定（`e2e/tests/event.spec.ts`）— 30 日後の相対日時に変更
- [ ] 申請者が UI から PRIVATE コミュニティに参加申請する入口が無い（一覧は PUBLIC のみ、非 ACTIVE には詳細が 404）。PENDING のまま申請を取り下げる UI も無い。招待リンク等の導線を設けるか決める（E2E の承認テストは申請を API で行っている）

### レビュー指摘（承認フロー修正時、差分外）
- [ ] `require_community_role`（`community/access.rs`）がロールだけを見て status を確認しない。一覧の `can_review_applications`（ACTIVE かつ管理ロール）と判定を揃える。ロール変更 API が無い現状では実害なし
- [ ] `reject` はサーバ側で対象の status を見ないため、API 直叩きで ACTIVE メンバー（ADMIN 含む）も削除できる。UI は PENDING のみに絞った。サーバ側も PENDING に限定するか決める（§2 の `reject` 項目と同じ論点）
- [ ] ADMIN にもメンバー一覧で PENDING が返るが、UI の承認／拒否ボタンは OWNER のみ表示（§5 の要確認と同じ論点）
- [ ] frontend は自分のメンバー情報（`me`）をページングされたメンバー一覧（既定 20 件）から探している。PENDING を先頭に並べたため、承認待ちが 20 件以上たまるとオーナー自身が 1 ページ目から外れ、承認ボタンが消えて「参加する」が出る（21 人目以降の ACTIVE メンバーには従来から同じ症状）。自分のメンバー情報を専用に取得する API / 処理を設ける
- [ ] `backend/tests/api.rs` が 1300 行超でファイル上限（800 行）を超過。コンテキストごとに分割する

### 開発環境
- [ ] trunk はビルドに失敗しても既存の `frontend/dist` を配信し続けるため、E2E が古い UI に対して黙って通る。webServer を `trunk build && trunk serve` にする等で失敗を顕在化させるか決める
- [ ] Dockerfile で固定した `wasm-bindgen-cli` の版（0.2.130）を `Cargo.lock` の wasm-bindgen 更新時に揃える

### 要確認（監査の指摘。旧挙動が不明なものを含む）
- [ ] ログアウト後に `/login` へ遷移しない（E2E のテスト名は「リダイレクトされる」だが URL を検証していない）
- [ ] 承認／拒否ボタンの表示が OWNER のみ。backend は ADMIN にも許可しており非対称
- [ ] コミュニティ作成・イベント作成・マイコミュニティ画面に未ログイン時のリダイレクトがない
- [ ] `openapi.rs` が実装とずれている: reject の 422 `CANNOT_REJECT_OWNER`、イベント一覧の 404 / 400、password の 72 バイト上限、パス/クエリの 400 が未記載
- [ ] テストの穴: ドメインイベント発行の検証、PENDING からの脱退、reject の `MEMBER_NOT_FOUND`
- [ ] wiki の記述が旧構成のまま（`require_community_role` の所在が `access.rs`、`TRUST_PROXY`、`error_mappings.rs`、統合テスト件数 26 → 47 など）。`/wiki sync` で更新する

移行時に入れた意図的な挙動変更は、decision-log の「意図した挙動の差異」「レビュー指摘への対応」を参照。
