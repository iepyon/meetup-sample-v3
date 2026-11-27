# Wiki Index

最終更新: 2026-11-27 (5891e84 + Rust 移行の未コミット作業ツリー)

運用ルールは [SCHEMA.md](SCHEMA.md)、操作は `/wiki` スキル（compile / ingest / query / lint / sync）。

## Contexts

- [auth (backend)](contexts/backend-auth.md) — アカウント登録・ログイン・JWT 発行と現在アカウント取得
- [community (backend)](contexts/backend-community.md) — コミュニティ CRUD とメンバーシップ管理（参加・脱退・承認・拒否）。UseCase は command / query 関数、メンバー一覧は Read モデル
- [event (backend)](contexts/backend-event.md) — コミュニティ配下のイベント作成（DRAFT）とイベント一覧
- [shared（共有カーネル）](contexts/backend-shared.md) — エラー型・JWT・Event Bus・手書き検証・middleware・OpenAPI など横断基盤
- [infrastructure (backend)](contexts/backend-infrastructure.md) — sqlx の SQLite 接続とスキーマ（schema.sql）の冪等適用、テストでのインメモリ DB
- [auth (frontend)](contexts/frontend-auth.md) — ログイン・登録画面と認証状態管理（Auth コンテキスト / token / api クライアント）
- [community (frontend)](contexts/frontend-community.md) — コミュニティ・イベント・メンバーの一覧/詳細/作成画面と共有 UI・ルート・proxy

## Domain

bounded context 別サブフォルダで整理（横断ページは domain/ 直下）。

### auth

- [Account 集約](domain/auth/account.md) — 認証の主体となるアカウント。登録時にメール重複を禁止

### community

- [Community 集約](domain/community/community.md) — コミュニティ本体。作成時にオーナーメンバーを同時生成、カテゴリと公開設定を持つ
- [CommunityMember 集約](domain/community/community-member.md) — コミュニティへの所属。role（OWNER/ADMIN/MEMBER）と status（PENDING/ACTIVE）を持つ
- [メンバーシップのロールと権限](domain/community/membership-rules.md) — 誰が何をできるか — OWNER/ADMIN/MEMBER の権限と PUBLIC/PRIVATE の違い

### event

- [Event 集約](domain/event/event.md) — コミュニティ配下のイベント。生成時は必ず DRAFT、日時バリデーション付き
- [イベントのライフサイクル](domain/event/event-lifecycle.md) — EventStatus の現状 — 遷移関数は未実装、DRAFT のみ生成される。公開/終了/中止の遷移条件は設計のみ

### participation（設計のみ・未実装）

- [Participation 集約](domain/participation/participation.md) — 参加申込の状態（申込/待ち/確定/キャンセル）。満席時のキャンセル待ちを担う CORE
- [キャンセル待ちと自動繰り上げ](domain/participation/waitlist-promotion.md) — キャンセル時の先頭自動繰り上げ。発火条件（previousStatus）と暴発ガード

### checkin（設計のみ・未実装）

- [CheckIn 集約](domain/checkin/checkin.md) — 当日来場の記録。承認済みのみ・1 参加 1 回

### notification（設計のみ・未実装）

- [Notification 集約](domain/notification/notification.md) — 通知レコードと 6 種の宛先ルール。編集通知は意図的に無し（D5）

### 横断

- [ドメインイベント一覧](domain/domain-events.md) — 実装済み 7 イベント（publish のみ・購読者ゼロ）と、設計のみの参加/通知イベント・ポリシー連鎖
- [ユビキタス言語（用語集）](domain/glossary.md) — ドメイン用語の日本語⇔コード上の英語名の対応表

## Patterns

- [Result<T, E>](patterns/result-type.md) — 例外を投げず成功/失敗を型で表現する。std の Result + コンテキストごとのドメインエラー enum
- [Branded Types（Rust 版では未採用）](patterns/branded-types.md) — 旧 TS 実装の型安全な ID。Rust 版は ID を String（UUID）で扱い、境界で uuid_param 検証
- [Discriminated Union Errors](patterns/discriminated-union-errors.md) — エラーをユースケースごとの enum で表し、errors.rs の From<…> for ApiError で HTTP に変換する
- [Schema-derived Types（列挙型の単一定義）](patterns/schema-derived-types.md) — 列挙値は string_enum! マクロの 1 箇所の定義から型・文字列定数・parse を導出する
- [手書き検証と手書き OpenAPI](patterns/openapi-driven-validation.md) — shared/validation.rs の手書き検証（400 VALIDATION_ERROR）と shared/openapi.rs の手書き OpenAPI JSON
- [Event Bus](patterns/event-bus.md) — コンテキスト間連携をドメインイベントで疎結合化する EventBus<E>（現状は publish のみ）
- [Readonly Interface](patterns/readonly-interface.md) — エンティティを振る舞いを持たない struct + 新しい値を返す遷移関数で不変的に扱う

## Infra

- [Docker 開発環境](infra/docker-dev.md) — 単一常駐コンテナに exec する docker-dev.sh の仕組み（cargo / trunk / E2E 用 npm）と WITH_CLAUDE
- [worktree 運用](infra/worktree.md) — worktree ごとに独立 Compose プロジェクト・独立ポートで main と同時起動する仕組み
- [CI・品質ゲート](infra/ci-quality-gates.md) — 3 層の品質ゲート（lefthook pre-commit / GitHub Actions / d review の中身）。cargo fmt / clippy / test が中核
- [AI hooks](infra/ai-hooks.md) — Claude Code 用 3 本の hook の発火タイミングと、lefthook / CI との役割分担
