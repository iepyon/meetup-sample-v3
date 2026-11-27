# E2E を Rust（chromiumoxide）に移行し、npm を撤去する

日付: 2026-11-27
種類: arch
ステータス: 承認済み

## コンテキスト

アプリ本体の Rust 全面移行（[arch-20261127-rewrite-in-rust.md](./arch-20261127-rewrite-in-rust.md)）では、E2E（Playwright / TypeScript、14 件）を**そのまま維持**していた。その結果、リポジトリに残る TypeScript と npm の用途は E2E だけになった。

- npm workspace（ルート `package.json` / `package-lock.json` / `e2e/package.json`）、prettier（lefthook の `e2e-prettier` と AI hook の `prettier-format.sh`）、`npm install` を監視する AI hook `dep-install-guard.sh` は、どれも E2E の TS のためだけに存在していた
- Docker には Playwright 用のブラウザキャッシュ（`PLAYWRIGHT_BROWSERS_PATH`）、依存ライブラリ群、`node_modules` の named volume 2 本が必要だった
- Playwright の `webServer` は backend（3000）と `trunk serve`（5173）を固定ポートで起動し、`reuseExistingServer: true` だった。このため `d dev` と同時に動かすと、レート制限が有効な dev サーバを再利用して後半のテストが失敗する、という運用上の落とし穴があった
- `lefthook install` は npm の `prepare` 経由で走っていた

## 決定

E2E を Rust のクレート `e2e/`（`meetup-e2e`、workspace メンバー）に移行し、npm を完全に撤去する。

1. **ブラウザ操作は chromiumoxide 0.9**（Chrome DevTools Protocol を直接話す）。chromedriver などの中間プロセスを持たない
2. **サーバは同一プロセスで起動する**。テストごとに `create_app_with_frontend` をインメモリ SQLite・`127.0.0.1:0`・レート制限無効で立て、`trunk build --dist ../target/e2e-dist` 済みの SPA を同一オリジンで配信する。本番と同じ配信経路を通す
3. **Playwright 風の Locator を自前で持つ**（`e2e/src/page.rs` + `locator.js`）。`get_by_role` / `get_by_label` / `get_by_text` と自動待機（10 秒ポーリング）、strict mode 相当の一意性チェックを持つ
4. テストは `#[ignore]` 付きにする。`cargo test --workspace`（`d test` / pre-commit / CI quality）はブラウザ無しで通し、clippy / fmt は E2E クレートにも効かせる。`d e2e` と CI の `e2e` ジョブが `-- --ignored` で実行する
5. 14 件はテスト名（日本語）を 1:1 で引き継ぐ
6. **npm を撤去する**: ルート / e2e の `package.json` と lock、lefthook の `e2e-prettier`、AI hook の `dep-install-guard.sh`（+ `.test.sh`）と `prettier-format.sh`、compose の `node_modules` volume を削除する。`d install` は `lefthook install` を明示的に呼ぶ。Node は Claude Code CLI 用としてイメージに残す
7. ブラウザは Docker では Debian の `chromium` パッケージ（arm64 / amd64 両対応）、CI ではランナーにプリインストールの Google Chrome を使う

## 理由と代替案

**採用した理由:**

- 言語とツールチェーンを Rust に一本化でき、npm・prettier・2 本の hook という E2E のためだけの保守対象がまるごと消える
- 同一プロセス起動にすると、テストごとに DB とポートが分離される。`d dev` との競合、`reuseExistingServer`、`RATE_LIMIT_DISABLED` の受け渡し、`e2e.db` の残骸といった落とし穴が構造的に無くなり、worktree 間で並列実行もできる
- backend の `create_app_with_frontend` を直接使うので、HTTP 統合テストと同じ組み立て方で E2E を書ける

**検討した代替案:**

- **Playwright を維持**: 最も成熟しているが、npm と TS が E2E のためだけに残り続ける。今回の目的（Rust への一本化）に反する
- **fantoccini / thirtyfour（WebDriver）**: chromedriver を別プロセスで起動・版合わせする必要があり、可動部品が増える
- **`trunk serve` + `cargo run` をサブプロセスで起動**: 旧構成に近いが、固定ポートとプロセス管理の問題が残る。同一プロセス起動で本番の配信経路を通せるため不要と判断した
- **`required-features` で E2E を除外**: `--all-targets` の clippy からも外れて lint が効かなくなるため、`#[ignore]` を選んだ

## 影響

- Playwright の高機能（トレース、UI モード、アクセシビリティツリーの正確な計算）は失う。Locator は DOM からの近似（role / アクセシブルネーム / label）なので、複雑な ARIA には対応しない。必要になったら `locator.js` を拡張する
- 失敗時の手がかりは panic メッセージ（最後の状態とページ本文の先頭）のみ。スクリーンショットは未対応
- E2E クレートの依存（chromiumoxide の CDP 型定義など）で、`cargo clippy --workspace --all-targets` と `cargo test --workspace` の初回ビルドが重くなる
- compose に `init: true` を追加した。PID 1 が `sleep` のままだと、終了した Chromium の子プロセスが回収されずゾンビとして溜まるため
- 既存のコンテナは `d build` → `d up` で作り直す必要がある（`chromium` の導入と volume 構成の変更のため）。不要になった `node_modules_*` volume は手動で `docker volume rm` する
- 既存の decision log・plan・`docs/wiki/log.md` の過去エントリは時点の記録として改変しない
