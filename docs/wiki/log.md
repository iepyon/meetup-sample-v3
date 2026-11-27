# Wiki Operation Log

追記専用。形式は [SCHEMA.md](SCHEMA.md) 参照。最新エントリの HEAD SHA が次回 `/wiki sync` の差分起点。

## [2026-11-27] ingest | E2E を Playwright から Rust（chromiumoxide）へ移行し npm と hook 2 本を撤去 (HEAD: 52810be)

- 出典: `docs/decision-logs/arch-20261127-e2e-in-rust.md` と、作業ツリー上の `e2e/`（`src/server.rs`・`src/lib.rs`・`CLAUDE.md`）・`scripts/docker-dev.sh`・`scripts/docker-worktree.sh`・`docker-compose.yml`・`Dockerfile`・`.github/workflows/ci.yml`・`.claude/settings.json`。HEAD の 52810be には未コミットのため、突合対象は作業ツリー
- 更新: infra/ai-hooks.md（hook 3 本 → 1 本。dep-install-guard / prettier-format の撤去理由を追記、`verified` を 52810be に更新。旧値 5891e84 はリポジトリに存在しない）, infra/ci-quality-gates.md（e2e ジョブを trunk build + `cargo test -p meetup-e2e -- --ignored` に、③ の対象からの依存追加を削除）, infra/docker-dev.md（named volume 4 本 → 2 本、ベースイメージに chromium、`PLAYWRIGHT_BROWSERS_PATH` 削除、`d install` / `d e2e` の中身、sources を `e2e/src/*` に差し替え）, infra/worktree.md（volume から node_modules を削除、`e2e` コマンドと `install` の `lefthook install` を追記）, contexts/frontend-auth.md（Playwright E2E → Rust E2E）, index.md
- `verified` は ai-hooks.md のみ更新。他のページは変更箇所だけを突合したため据え置き（既存値 5891e84 はリポジトリに存在しないので、次回 `/wiki sync` でページ全体を突合して更新する）
- docker-dev.md の「d dev と d e2e の併用（Gotcha）」節（[2026-10-13] の ingest で追加）は、E2E が空きポート・インメモリ SQLite・専用 SPA 出力先で動くため問題自体が解消。節は併用可である旨の説明に書き換えた
- wiki 外: `README.md`・`docs/docker-usage.md`・`docs/remaining-items.md`・`.claude/agents/{code-reviewer,test-analyzer}.md`・`tutorial/1day-tutorial.md`（hook の参照箇所のみ）も同じ事実に合わせて更新
- 矛盾検出: なし（`docs/conventions/testing.md`・AGENTS.md は Rust E2E の記述に更新済みであることを確認）

## [2026-11-27] ingest | TypeScript から Rust（axum + sqlx + Leptos）への全面移行に伴う全ページの更新 (HEAD: 5891e84)

- 出典: `docs/decision-logs/arch-20261127-rewrite-in-rust.md` と、作業ツリー上の Rust 実装（`backend/src/{auth,community,event,shared,infrastructure}`・`backend/tests/api.rs`・`frontend/src/`）。HEAD の 5891e84 には Rust 実装が**未コミット**のため、`verified` は 5891e84 にしているが突合対象は作業ツリー（コミット後に `/wiki sync` で再確認すること）
- 更新（contexts）: backend-auth, backend-community, backend-event, backend-shared, backend-infrastructure, frontend-auth, frontend-community — 旧技術（Express / Prisma / zod / Vitest / Vite / React）の記述を Rust 構成のファイルパス・型名・関数名に置換
- 更新（patterns）: result-type（std の Result + ドメインエラー enum）, branded-types（Rust 版では未採用。ID は String）, discriminated-union-errors（enum + `From<…> for ApiError`）, schema-derived-types（`string_enum!` マクロ）, openapi-driven-validation（手書き検証・手書き OpenAPI。ページ名は据え置き）, event-bus（`EventBus<E>`）, readonly-interface
- 更新（domain）: account, community, community-member, membership-rules, event, event-lifecycle, domain-events, glossary — 業務ルールは変更せずコード位置・型名・強制ポイントのみ修正。community に「一覧は PUBLIC のみ」を追記（旧 TS 実装は repository が visibility 条件を無視して非公開も返していたバグ。Rust 移行で修正）。participation / checkin / notification は「Branded 想定」→「UUID の String 想定」のみ
- 更新（infra）: docker-dev, worktree, ci-quality-gates（cargo fmt / clippy / test。層依存・循環依存・複雑度・カバレッジ 80% は**未移植**と明記）, ai-hooks（prettier-format が `.rs` 対象外である点を追記）
- 更新: index.md（説明文・日付）。`docs/docker-usage.md` も新コマンド体系に更新（wiki 外）
- 記述上の注意: パターンページ名（branded-types.md 等）はリンク維持のため据え置き。Branded Types は「未採用」ページとして残した
- 発見（コード上の事実）: `GET /communities/{id}/events` はコミュニティの存在・公開設定を確認しない（DRAFT 可視性のみ）。`reject_member_command` は対象の status / role を検査しない。いずれも現状の記述として backend-event / community-member に記載
- 矛盾検出: SCHEMA.md の例示パス（`backend/src/community/models/community.ts` 等）が旧 TS 構成のまま。SCHEMA.md は人間所有のため未編集

## [2026-10-13] ingest | d dev 起動中の d e2e がレート制限で失敗する Gotcha (HEAD: 2ca6d307)

- 出典: 動作確認セッションでの E2E 失敗（3/11 件）と、dev サーバ停止後の再実行（11/11 成功）
- 更新: infra/docker-dev.md（Gotcha 節を追加、sources に `e2e/playwright.config.ts` と `rate-limit.middleware.ts` を追加）
- `verified` は据え置いた。既存値 `bc314949` がリポジトリに存在せず、ページ全体を突合する起点が取れないため
- 矛盾検出: なし

## [2026-09-15] ingest | AI ハーネスを Claude Code 専用に整理（Copilot 併用と重複 hook を撤去） (HEAD: cc1d552)

- 出典: `docs/decision-logs/arch-20260915-claude-code-only-harness.md`
- 更新: infra/ai-hooks.md, infra/ci-quality-gates.md, index.md
- ai-hooks.md: hook 5 本 → 3 本。「両ハーネス対応の仕組み（harness-lib.sh）」節を「hook に載せないもの（重複排除の方針）」節へ置換し、「登録場所の違い」を `.claude/settings.json` 単独の「登録場所」に改題。deny リストへの言及を削除
- ci-quality-gates.md: 3 層の ③（AI hooks）が ①② と重複しない AI 固有のリスクに限定される旨を追記
- sources 変更: ai-hooks.md から `.github/hooks/hooks.json` を除去（ファイル削除に伴う lint 対象の整合）
- 矛盾検出: なし（表の内容が `.claude/settings.json` の登録 3 本と一致することを確認済み）

## [2026-09-10] ingest | AGENTS.md 削減に伴う逆参照の整合 (HEAD: bc314949)

- 出典: `docs/plans/agents-md-slim-20260910.md` Phase 5-6（AGENTS.md 削減計画の PR 2）
- 更新: infra/docker-dev.md, infra/worktree.md, index.md
- AGENTS.md から Commands の詳細が `docs/docker-usage.md` へ移ったため、「コマンド一覧は AGENTS.md」という逆参照を `docs/docker-usage.md`（早見表 §4.4 / レシピ §4.5 / worktree §6）へ差し替えた
- ページ本文の事実記述は変更なし（cross-reference のみ）。`verified` は SCHEMA.md の規定に従い更新
- 矛盾検出: なし

## [2026-09-10] ingest | hook 発火タイミング表を AGENTS.md から ai-hooks.md へ移設 (HEAD: aaf3bcb7)

- 出典: `docs/plans/agents-md-slim-20260910.md` Phase 2（AGENTS.md 削減計画の PR 1）
- 更新: infra/ai-hooks.md（発火タイミング一覧の節を新設）, index.md
- 循環参照の解消: `ai-hooks.md` が「一覧表は AGENTS.md 参照」と書き、AGENTS.md はどこも指していない状態だった。表を wiki 側に移して自己完結させた
- 本エントリ時点では AGENTS.md 側の表は未削除（PR 2 で削除予定）。一時的に重複するが、内容は同一
- 矛盾検出: なし（表の内容は `.claude/settings.json` / `.github/hooks/hooks.json` と一致することを確認済み）

## [2026-07-12] ingest | EventStorming セッション meetup-20260712 の決定を domain に反映 (HEAD: 1929475)

- 出典: `docs/eventstorming/eventstorming-20260712-1141.dml.yaml`（pocket-modeling からのスナップショットをリポジトリへコピー）と `docs/issues-20260712.md`
- 新規: domain/participation/participation.md, domain/participation/waitlist-promotion.md, domain/checkin/checkin.md, domain/notification/notification.md（すべて「未実装（設計のみ）」区分で作成）
- 更新: domain/event/event.md（D3 定員減員ガード）, domain/event/event-lifecycle.md（公開/終了/中止/編集の遷移条件を設計のみで追記）, domain/glossary.md（Participation/CheckIn/Notification 集約・ParticipationStatus/NotificationType enum・用語弁別 2 件）, domain/domain-events.md（設計のみのイベント・ポリシー連鎖表）, index.md
- 意思決定の反映: D1（自動繰り上げ）/ D2（繰り上げ通知は本人のみ）/ D3（定員減員ガード）/ D4（中止通知に APPLIED を含む）/ D5（編集通知は意図的に無し）/ D6（CMD 名は実装と同名維持）。未決 Q4/Q5 は waitlist-promotion.md に明記
- flow ページ（domain/<context>/flows/）は今回見送り。contexts/ は対応 BC のコードが無いため新設せず
- 矛盾検出: なし（既存 wiki の実装状況記述は正確。participation/checkin/notification は元々コード不在で、今回のページはすべて設計のみ区分）

## [2026-07-12] ingest | domain ページを新テンプレートに準拠化（強制ポイント・実装状況 3 区分） (HEAD: 1709be72)

- 更新: domain/auth/account.md, domain/community/community.md, domain/community/community-member.md, domain/event/event.md, domain/event/event-lifecycle.md, domain/domain-events.md, domain/glossary.md
- 不変条件ごとに強制ポイント（factory / usecase / middleware / DB 制約）を明記。DB unique 制約を prisma schema で確認し sources に追加（Account.email・Community.name は usecase チェック + DB unique の二重、CommunityMember は複合 unique。コミュニティ作成上限 10 のみ DB 制約なし）
- event / event-lifecycle / domain-events に「現状の実装」（実装済み / 先行定義）を明示、glossary に「紛らわしい用語の弁別」欄を追加
- 矛盾検出: なし（eb6fe821..HEAD に backend コード差分なし）

## [2026-07-12] reorganize | domain/ を bounded context 別サブフォルダに再編成 (HEAD: eb6fe821)

- ユーザー指示により SCHEMA.md を更新（domain/ は BC 別サブフォルダ、横断ページのみ直下）
- 移動: account.md → auth/、community.md・community-member.md・membership-rules.md → community/、event.md・event-lifecycle.md → event/
- 直下に残留（横断）: glossary.md, domain-events.md
- 全ページの相対リンクと index.md を新パスに更新、verified を eb6fe821 に更新
- 矛盾検出: なし

## [2026-07-12] compile | 初期構築（全カテゴリ） (HEAD: 51b45439)

- 新規: contexts 7 ページ（backend-auth, backend-community, backend-event, backend-shared, backend-infrastructure, frontend-auth, frontend-community）
- 新規: domain 8 ページ（account, community, community-member, event, membership-rules, event-lifecycle, domain-events, glossary）
- 新規: patterns 7 ページ（result-type, branded-types, discriminated-union-errors, schema-derived-types, openapi-driven-validation, event-bus, readonly-interface）
- 新規: infra 4 ページ（docker-dev, worktree, ci-quality-gates, ai-hooks）
- 調査: codebase-explorer サブエージェント 4 体（backend contexts / domain / frontend+patterns / infra）に並列委譲
- 特記事項（調査で判明したドキュメント未記載の事実）:
  - Event Bus は publish のみで subscriber がコード全体にゼロ（event-bus.md, domain-events.md に明記）
  - event コンテキストは community のリポジトリ実装・middleware を直接 import（依存方向 event → community）
  - EventStatus の DRAFT 以外への遷移関数は未実装（event-lifecycle.md）
  - Vite proxy は AGENTS.md 記載の 3 つでなく実際は 6 パス（frontend-community.md）
  - dependency-cruiser ルールの participation / checkin BC は将来用でソースに実在しない（ci-quality-gates.md）
- 矛盾検出: なし（上記は「ドキュメントの記載漏れ・先行定義」であり決定との矛盾ではない）
