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
- [ ] セキュリティヘッダ: `X-Content-Type-Options` / `X-Frame-Options` / `Referrer-Policy` と JSON 応答の CSP は 2026-11-27 に対応済み。残りは HTML（SPA・/docs）の CSP と HSTS（→ §6）
- [x] `JWT_SECRET` の最小長は警告のみ。必要なら起動拒否に変更（E2E / 開発用の短い値との兼ね合い） — 2026-11-27 に 32 バイト未満は起動拒否へ変更（§6）
- [x] `reject` は ACTIVE な一般メンバーも削除できる（OWNER のみ保護）。PENDING のみ許可にするか決める — 2026-11-27 にサーバ側も PENDING のみに限定（ACTIVE は 422 `MEMBER_NOT_PENDING`）
- [ ] `npm install -g @anthropic-ai/claude-code`（Dockerfile）のバージョン未固定

### 品質ゲート
- [ ] カバレッジ 80% ゲート（`cargo llvm-cov --fail-under-lines 80` を CI / `d review` に追加）
- [ ] 層依存・循環依存・複雑度の自動検査（旧 dependency-cruiser / madge 相当。`cargo-modules` 等）

### テスト
- [ ] E2E に未ログイン時のリダイレクトを追加（PRIVATE コミュニティの承認／拒否は 2026-11-27 に追加済み）
- [ ] Leptos ページ/コンポーネントの単体テスト（現状は E2E 依存）
- [ ] 1 テストに複数検証が入っているもの（`api.rs` の参加/脱退、承認/拒否など）を「1 基準 1 テスト」に分割
- [ ] レート制限テストが実時間・固定回数（61 回など）に依存。時刻注入でウィンドウ復帰も検証
- [x] `main.rs` の `JWT_SECRET` 判定を純粋関数に切り出してテスト — `shared::auth::validate_jwt_secret` に切り出し単体テストを追加

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
- [x] `require_community_role`（`community/access.rs`）がロールだけを見て status を確認しない。一覧の `can_review_applications`（ACTIVE かつ管理ロール）と判定を揃える。ロール変更 API が無い現状では実害なし — 2026-11-27 に ACTIVE 条件を追加（`has_active_role`）
- [x] `reject` はサーバ側で対象の status を見ないため、API 直叩きで ACTIVE メンバー（ADMIN 含む）も削除できる。UI は PENDING のみに絞った。サーバ側も PENDING に限定するか決める（§2 の `reject` 項目と同じ論点） — 2026-11-27 に PENDING のみに限定
- [ ] ADMIN にもメンバー一覧で PENDING が返るが、UI の承認／拒否ボタンは OWNER のみ表示（§5 の要確認と同じ論点）
- [ ] frontend は自分のメンバー情報（`me`）をページングされたメンバー一覧（既定 20 件）から探している。PENDING を先頭に並べたため、承認待ちが 20 件以上たまるとオーナー自身が 1 ページ目から外れ、承認ボタンが消えて「参加する」が出る（21 人目以降の ACTIVE メンバーには従来から同じ症状）。自分のメンバー情報を専用に取得する API / 処理を設ける
- [x] `backend/tests/api.rs` が 1300 行超でファイル上限（800 行）を超過。コンテキストごとに分割する — 2026-11-27 に `api.rs` / `system.rs` / `security.rs` + `common/` に分割

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

## 6. セキュリティ点検で見送ったもの（2026-11-27）

backend / frontend・実行環境の 2 本のセキュリティレビューのうち、2026-11-27 のコミットで直さなかったもの。

### 決定待ち
- [x] `JWT_SECRET` を 32 バイト未満で起動拒否にするか。する場合、compose の既定値（17 バイト）を `.env` 必須（`${JWT_SECRET:?}`）にするか、開発用の既定値を延ばすか（E2E の値は 32 バイト以上に変更済み） — 起動拒否 + compose は `${JWT_SECRET:?}` で `.env` 必須に決定・対応済み
- [x] `.claude/settings.json` の許可リスト（`Write` / `Edit` / `Read` / `WebFetch` が無条件）を絞るか。`scripts/**`・`.claude/**`・`lefthook.yml` への書き込みを ask にする等 — `scripts/**`・`.claude/**`・`.git/**`・`lefthook.yml` への書き込みを ask に変更済み（Read / WebFetch は現状維持）
- [x] `WITH_CLAUDE=1` 時の `~/.claude` マウントがメインでは rw。コンテナからホストの hook 設定を書き換えられる。`:ro` にするか — main・worktree とも `:ro` に変更済み
- [ ] secret-leak-detector の matcher が `Bash` のみ（Read / Grep / WebFetch の出力は未検査）。広げるか（settings.json の変更）

### backend
- [ ] HTML の CSP: trunk のインラインのモジュールローダー（hash / nonce が必要）と /docs の CDN（jsdelivr）に対応した CSP を設計する。HSTS は TLS 終端の構成に合わせて付ける
- [ ] レート制限の作り: バケット掃除が全件走査（O(n)）で、上限（5 万件）到達時は新規 IP を全拒否（fail-closed）。期限順キューや `governor` に置き換える。bcrypt の同時実行数を Semaphore で制限する。アカウント単位のログイン失敗制限を入れる
- [ ] PUBLIC コミュニティでは参加するだけで全メンバーの accountId が見える。frontend が自分を特定する用途だけなので、自分のメンバー情報の専用 API（§5 の `me` の項目）に切り替えて一覧からは外す
- [ ] 名前・タイトルに制御文字や RTL 制御文字（Cc / Cf）を入れられる。パスワード・説明と共用している `required_string` を分けて拒否する
- [ ] コミュニティ作成数の上限チェックと INSERT が同一トランザクションでない（並行リクエストで上限を超え得る）
- [ ] 存在しない PRIVATE コミュニティと非メンバーで join / leave の応答が異なる（UUID のため実害は小さい。招待制の検討と合わせる）
- [ ] 登録 API でメールアドレスの登録有無が分かる（409）。根本対策はメール確認方式
- [ ] セキュリティイベント（ログイン失敗・403・429）のログがない。`tracing` を導入する
- [ ] bcrypt のコストが 10（OWASP の最低値）。12 を検討
- [ ] `/docs` と `/openapi.json` を本番で無効化できる設定、`RATE_LIMIT_DISABLED` を本番で無視するガード
- [ ] メールアドレスは登録時に小文字化し `lower(email)` で照合するようにしたが、DB の UNIQUE は大文字小文字を区別したまま。既存データの大小違いの重複を確認・解消してから `lower(email)` に UNIQUE インデックスを張る（現状は非 UNIQUE の式インデックスのみ）
- [ ] 本番配信時、SPA へのページ遷移（`/communities/<id>` の HTML）も公開 GET のレート制限（60 回/分）を消費する。HTML 遷移を別枠にするか検討
- [ ] chunked（長さ不明）ボディの 415 判定を統合テストで固定する

### frontend
- [ ] ログアウトはクライアント側の破棄のみで、JWT はサーバ側で 24 時間有効なまま。失効リスト（jti）か短命トークン + リフレッシュ
- [ ] 401 を受けると保存済みトークンは消すが、`Auth` のメモリ上のログイン状態は残る（画面はログイン中の表示のまま）。401 を Auth 側で受けて `logout` する仕組みにする

### 開発環境・サプライチェーン
- [ ] Dockerfile: ベースイメージ `node:26-bookworm` を digest で固定していない。`git config --system --add safe.directory '*'` をマウント先に限定する。trunk が取得する tailwindcss / wasm-opt の検証方法を確認する
- [ ] CI: 公式 Action（`actions/checkout` 等）もタグ指定。サードパーティと合わせて commit SHA に固定する
- [ ] dep-install-guard を `bash -c 'npm install X'`・`/usr/bin/npm`・`npm --prefix`・`npx`・`pnpm` / `yarn` / `bun` で回避できる
- [ ] コンテナから rw マウントのリポジトリ経由で `scripts/`・`lefthook.yml`・`.git/hooks` を書き換えられ、ホストの次の commit で実行される。ビルド・テスト用の経路と編集の経路を分けるか検討
- [ ] lefthook の `{staged_files}` を引用符なしで渡す形は、lefthook がファイル名をエスケープすることに依存する（ホストの 2.2.2 で確認済み）。コンテナ内は 1.10.4 のため `min_version` を一律には上げられない。版をそろえて `min_version` を設定する
- [ ] secret-leak-detector の誤検知（`API_KEY=your-api-key` 等のプレースホルダ、`sk-` で始まる一般の文字列）と検知漏れ（JSON / 引用符付きの `"X_API_KEY": "…"`、小文字の `api_key=`）
- [ ] シェルスクリプトの `$COMPOSE` / `$EXEC` が未クォート（空白を含むパスで壊れる）。`docker-worktree.sh` の worktree 名に `../` を含められる

