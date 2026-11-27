# Wiki Schema — LLM Wiki の運用ルール

このディレクトリは [Karpathy の LLM Wiki パターン](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f)に基づく、**LLM が構築・保守するコードベース + ドメイン知識 Wiki**。

- **本ファイル（SCHEMA.md）は人間が所有する**。LLM は読むだけで編集しない
- **それ以外の `docs/wiki/` 配下は LLM が所有する**。人間は読む・レビューするが、更新は `/wiki` スキル経由で LLM が行う
- Raw sources（コード本体・`docs/decision-logs/`・`docs/conventions/`・DML 成果物）は読むだけで編集しない

## ディレクトリ構成とページ種別

```
docs/wiki/
├── SCHEMA.md      # 本ファイル（人間所有）
├── index.md       # 全ページ目録。Query 時の最初の読み込み対象
├── log.md         # 追記専用の操作ログ
├── contexts/      # bounded context ページ
├── domain/        # ドメイン知識ページ（bounded context 別サブフォルダ）
│   ├── auth/        # auth コンテキストの集約・ルール
│   ├── community/   # community コンテキストの集約・ルール
│   │   └── flows/     # 集約横断の業務フロー（各コンテキスト配下に置ける）
│   ├── event/       # event コンテキストの集約・ルール
│   ├── glossary.md      # コンテキスト横断（直下に置く）
│   └── domain-events.md # コンテキスト横断（直下に置く）
├── patterns/      # コアパターンページ
└── infra/         # インフラ・開発環境ページ
```

`domain/` 直下には bounded context 別のサブフォルダを作って整理する。複数コンテキストにまたがるページ（用語集・ドメインイベント一覧など）のみ `domain/` 直下に置く。新しい bounded context が増えたらサブフォルダを追加する。

| type | 置き場所 | 内容 |
|------|---------|------|
| `context` | `contexts/` | bounded context ごとの構造・責務・主要ファイル・他コンテキストとの連携（backend: auth/community/event/shared/infrastructure、frontend: auth/community） |
| `domain` | `domain/<context>/` | 集約ページ（不変条件・状態遷移・発行するドメインイベント）、業務ルールページ（ロールと権限、ライフサイクル等）。横断ページ（`glossary.md`＝ユビキタス言語、`domain-events.md`）は `domain/` 直下 |
| `flow` | `domain/<context>/flows/` | 集約横断の業務フロー（コマンド → 集約 → イベント → ポリシーの連鎖）。EventStorming / DML 成果物の SCENARIO・POLICY と対応させる。複数コンテキストにまたがる場合はフローの起点側コンテキストに置く |
| `pattern` | `patterns/` | コアパターンの目的・実装場所・使い方・アンチパターン（Result 型、Branded Types 等） |
| `infra` | `infra/` | Docker 構成、worktree 運用、CI/品質ゲート、AI hooks |

## ページのフロントマター

```yaml
---
type: context | domain | flow | pattern | infra
title: ページ名
description: 一行要約（index.md に転記される）
tags: [tag1, tag2]
sources:
  - backend/src/community/models/community.ts
  - docs/decision-logs/arch-20260406-bounded-context-split.md
verified: <commit-sha>
---
```

- `sources`: このページの根拠となるコードパス・decision-log。**issue（`docs/issues-*.md`）・EventStorming / DML 成果物（`docs/eventstorming/*.md`）も可** — コードに未実装だが決定済みの仕様は、根拠となる非コードソースを必ず sources に含める。lint の突合対象
- `verified`: ページ内容を最後にコードと突合したときの HEAD コミット SHA（短縮形 7 桁以上）。`git diff <verified>..HEAD -- <sources>` に差分があるページは陳腐化候補

## 執筆ルール

- 日本語で書く。コード識別子・ファイル名は原文のまま
- ファイル名は英語 kebab-case（例: `community-membership.md`）
- ページ間リンクは相対 Markdown リンク（例: `[Result 型](../patterns/result-type.md)`）
- 「なぜそうなっているか」は decision-log への相対リンクで接続する（内容の重複コピーはしない）
- AGENTS.md / conventions と同じ内容をコピペしない。参照リンクにし、Wiki には「そこに書かれていない統合・詳細」だけを書く
- コード参照は `ファイルパス:行番号` ではなく **ファイルパス + シンボル名**で書く（行番号はすぐ陳腐化するため）
- 1 ページ 150 行以内を目安。超えるなら分割する

## domain ページのテンプレート

### 実装状況の明示（domain / flow 全ページ共通）

区別なしに書かれた記述は「実装済み」と読まれる。先行定義・未実装を含むページには「現状の実装」セクションを必ず置き、次の 3 区分を明示する:

- **実装済み** — 動作するコードがある
- **先行定義** — enum・型・イベント定義だけ存在し、使う側が未実装（例: 遷移関数のない EventStatus 値、購読者のいないドメインイベント）
- **未実装（設計のみ）** — DML・issue にだけ存在する

### 集約ページ（`domain/<context>/*.md`）の必須セクション

1. **フィールド** — schema ファイル基準の表
2. **factory・遷移関数** — シグネチャと Result のエラー型
3. **状態遷移** — 状態を持つ場合はテキスト図。物理削除など「状態遷移でない終了」も明記する
4. **不変条件・業務ルール** — ルールごとに**強制ポイント**（models の factory / usecase / controller middleware / DB 制約のどれで守られているか）を明記する。DB 制約で守られていないルールはそれとわかるように書く
5. **現状の実装** — 上記の実装状況ルール（全部実装済みなら省略可）
6. **関連** — 関連ページへのリンク

### flow ページ（`domain/<context>/flows/*.md`）の必須セクション

1. **フロー** — コマンド → 集約 → イベント（→ ポリシー）の連鎖のテキスト図
2. **関与する集約・Read モデル** — 各集約ページへのリンク
3. **分岐・エラー経路** — 主要な失敗パターンとエラー型
4. **現状の実装** — 実装状況ルールに従う
5. **対応する DML** — 成果物ファイルと SCENARIO / POLICY 名（DML 由来の場合）

### glossary の運用

- 新しい enum 値・エラー型・ドメインイベントを追加する変更は `glossary.md`（イベントは `domain-events.md` も）への追記対象
- 紛らわしい用語は「弁別」欄で明示的に区別する（例: `Event` 集約＝勉強会イベント、ドメインイベント＝Event Bus に publish される事実の記録）

## 禁止事項

- `snapshot/*` ブランチをソースにしない（`git show/log/diff snapshot/...` を実行しない。CLAUDE.local.md 準拠）
- 機密ファイル（.env, credentials, *.pem, *.key）を読まない・引用しない
- `log.md` の過去エントリを書き換えない（追記のみ）
- lint は報告のみ。修正は人間の確認を経て ingest で行う

## index.md の形式

カテゴリ別に 1 ページ 1 行 + description の転記。LLM は ingest / compile のたびに更新する。

```markdown
# Wiki Index

最終更新: YYYY-MM-DD (<commit-sha>)

## Contexts
- [auth (backend)](contexts/backend-auth.md) — 登録・ログイン・JWT 発行

## Domain
- [Community 集約](domain/community/community.md) — コミュニティの不変条件と状態遷移
...
```

## log.md の形式

追記専用。各エントリに操作種別・対象・touched pages・その時点の HEAD SHA を記録する。`grep "^## \["` で履歴抽出できる形式を保つ。

```markdown
## [YYYY-MM-DD] ingest | <対象の要約> (HEAD: <sha>)
- 更新: contexts/backend-community.md, domain/glossary.md
- 新規: domain/event-lifecycle.md
- 矛盾検出: なし
```

最新エントリの HEAD SHA が、次回 `/wiki sync` の差分起点になる。
