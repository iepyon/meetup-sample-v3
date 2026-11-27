---
name: wiki
description: コードベース + ドメイン知識の LLM Wiki（docs/wiki/）を構築・保守・活用するスキル。`/wiki compile|ingest|query|lint|sync` 実行時、「Wiki を更新して」「Wiki に聞いて」「Wiki の健全性チェック」等の依頼時に使用する。アーキテクチャ・ドメイン知識に関する質問を受けたときは、コードを掘る前にまず query 操作で Wiki を参照する。
---

# Wiki — LLM が保守するコードベース + ドメイン知識 Wiki

`docs/wiki/` を構築・保守・活用する。運用ルール（ページ種別・フロントマター・執筆ルール・禁止事項）は **必ず最初に `docs/wiki/SCHEMA.md` を読んで**従うこと。

引数の第 1 語で操作を選ぶ: `compile` / `ingest` / `query` / `lint` / `sync`。引数なしで呼ばれた場合は、会話の文脈から操作を推定し、確認してから実行する。

## 共通ルール

- SCHEMA.md は編集しない（人間所有）
- `snapshot/*` ブランチをソースにしない。機密ファイルを読まない
- どの操作でも、ページを作成・更新したら **index.md を更新し、log.md にエントリを追記**する（query で読み取りのみの場合は log 不要）
- log.md エントリには現在の HEAD SHA（`git rev-parse --short=8 HEAD`）を記録する
- ページ更新時はフロントマターの `verified` を現在の HEAD SHA に更新する

## compile — 初期構築 / 全再構築

1. `docs/wiki/SCHEMA.md` を読み、ページ種別と執筆ルールを確認する
2. 調査用サブエージェント（Explore 等）に領域別調査を**並列**で委譲する（メインコンテキスト保護）。目安: ① backend contexts ② domain（集約・ルール・用語）③ frontend + patterns ④ infra
   - 各サブエージェントには「既存ページがあれば `verified` SHA 以降の差分（`git diff <verified>..HEAD -- <sources>`）だけを裏取りする」と明示的に指示する
3. 調査結果をもとに SCHEMA.md に従ってページ群を生成する
4. `index.md` を生成し、`log.md` に compile エントリを追記する
5. `/wiki lint` を実行してリンク切れ・孤立ページがないことを確認する

## ingest — 差分・新ソースの取り込み

対象指定あり（例: `/wiki ingest docs/decision-logs/arch-xxx.md`、DML 成果物、PR）ならそれを、なければ log.md 最新エントリの HEAD SHA 以降の変更を取り込む。

1. `log.md` の最新エントリから前回 SHA を取得し、`git log --stat <前回SHA>..HEAD` で変更を把握する（docs のみの変更・wiki 自身の変更は無視してよい）
2. 変更に関連するページを `index.md` と各ページの `sources` から特定する
3. 該当ページを更新する。判断基準:
   - **既存ページの編集** — 既存概念の属性・構造の変化
   - **新規ページの作成** — 他から参照される独立した新概念（新しい集約・新しいパターン・新しい bounded context）
   - decision-log 追加 → 関連する patterns / infra / domain ページの「なぜ」リンクを更新
   - `models/schemas/` や遷移関数の変更 → domain ページ（集約・glossary）を更新
   - `errors/` への新エラー型 — 新しい業務ルールの兆候 → 該当集約ページの不変条件・強制ポイントを更新
   - ドメインイベント型の追加・購読者（subscriber）の実装 → `domain/domain-events.md`（「購読者ゼロ」など現状を断定した記述は最優先で見直す）
   - `composition.ts` / 認可 middleware の変更 → 権限マトリクス（`domain/community/membership-rules.md`）
   - enum 値の追加・変更 → `domain/glossary.md`（紛らわしい用語は弁別欄も更新）
4. 既存ページと矛盾する変更を見つけたら、ページを新しい事実で更新し、log.md に「矛盾検出」として記録する
5. `index.md` 更新 + `log.md` に ingest エントリ追記（更新/新規ページ一覧・HEAD SHA）

### DML / EventStorming 成果物の取り込み

`/wiki ingest docs/eventstorming/<file>.md` の形で指定されたら:

1. 成果物のコンテキスト候補・集約候補・DML・用語集セクションを読む
2. SCENARIO / POLICY → `domain/<context>/flows/` の flow ページを作成・更新し、`sources` に成果物ファイルを追加する
3. 集約候補と既存集約ページ（実装）の差分は、集約ページの「現状の実装」に **未実装（設計のみ）** として記録する（実装済みかのように書かない）
4. 成果物の用語集（日本語⇔英語対応）を `domain/glossary.md` に統合する

## query — Wiki を起点にした質問応答

1. `index.md` を読み、質問に関連するページを特定する（フルスキャンしない）
2. 関連ページ（目安 5 ページ以内）を読み、**ページへの引用付き**で回答する
3. Wiki に無い知識が必要ならコード・decision-logs を掘って補い、回答する
4. ステップ 3 で得た知識が再利用価値を持つ場合（構造・ルール・パターンの発見）、該当ページに還元するか新規ページを作成し、log.md に記録する。一時的・タスク固有の情報は還元しない

## lint — 健全性チェック（報告のみ、自動修正しない）

1. **陳腐化**: 各ページの `verified` SHA と `sources` を突合し、`git diff --stat <verified>..HEAD -- <sources...>` に差分があるページを列挙する
2. **孤立ページ**: index.md 以外のどのページからもリンクされていないページ
3. **リンク切れ**: 存在しないページ・ファイルへの相対リンク
4. **矛盾**: decision-logs の最新の決定（ステータス・撤回）と食い違う記述（例: 撤回済み決定を前提にしたページ）
5. **ギャップ**: コードに存在するが Wiki にページがない bounded context・集約・パターン
6. **ドメイン整合**（grep で機械照合できる）: ① コード上のドメインイベント型一覧 vs `domain/domain-events.md` の表、② `models/schemas/` の enum 値 vs `domain/glossary.md` の表、③ `composition.ts` の `requireCommunityRole` 指定 vs 権限マトリクス（`domain/community/membership-rules.md`）
7. 結果を優先度順のレポートとして出力する。修正は行わず、ユーザー確認後に `/wiki ingest` で対応する

## sync — /loop 向けの冪等な定期更新

`「/loop /wiki sync」`（自律ペース）または `「/loop 30m /wiki sync」`（固定間隔）での定期実行を想定。

1. `log.md` 最新エントリの SHA と HEAD を比較する。**差分ゼロなら何もせず「変更なし」とだけ報告して終了**（log にも書かない）
2. 差分があれば ingest（手順は上記）を実行する
3. 続けて lint の簡易版（陳腐化 + リンク切れのみ）を実行し、問題があればレポートする

## コミット

Wiki の変更をコミットする場合は japanese-commit 形式で `docs: wiki を更新（<要約>） (docs)` とする。
