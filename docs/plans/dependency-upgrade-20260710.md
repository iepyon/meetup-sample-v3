# 依存関係アップグレード マイグレーション計画（2026-07-10）

## 背景

Node.js と依存ライブラリの最新化を目的に、2026-07-10 時点の npm registry / 公式リリースノートを調査した。
本計画は調査結果と移行手順をまとめたもの。**実作業は本計画の承認後に別途実施する。**

## 現状把握（調査結果）

### Node.js — engines と実行環境のズレあり（要修正）

| 指定箇所 | 現在値 | 問題 |
|---|---|---|
| `Dockerfile:1` | `FROM node:24-bookworm` | なし（Node 24 = Active LTS） |
| `.github/workflows/ci.yml` | `setup-node@v4` / `node-version: 24` | なし |
| `package.json` engines（ルート/backend/frontend/e2e） | `>=20.19.0` | **Node 20 は EOL 済み**。実行環境（24）と最低要件が乖離 |

Node リリース状況（2026-07 時点）:

- **Node 24 = Active LTS**（EOL 2028-04-30）← 実行環境はすでにこれ
- Node 22 = Maintenance LTS（EOL 2027-04-30）
- Node 26 = Current（2026-10-28 に LTS 化予定）
- **Node 20 = EOL 済み**

主要依存の engines 実測: vite `^20.19.0 || >=22.12.0`、jsdom / eslint `^20.19.0 || ^22.13.0 || >=24`、vitest `^20 || ^22 || >=24`、Playwright `>=18` — **Node 24 なら全パッケージ問題なし**。

### 依存ライブラリ — 主要依存はほぼ最新メジャーに到達済み

メジャー更新が存在するのは **TypeScript（6→7）** と **dependency-cruiser（17→18）** の 2 件のみ。
以下は未リリース（追随不要）: **Prisma 8 / Zod 5 / React 20 / react-router v8 / Vite 9 / Tailwind 5**。

#### minor / patch 更新（低リスク、caret 範囲内）

| パッケージ | 現在 | 最新 | 対象 workspace |
|---|---|---|---|
| eslint | 10.5.0 | 10.6.0 | backend / frontend |
| typescript-eslint | 8.61.1 | 8.63.0 | backend / frontend |
| vitest / @vitest/coverage-v8 | 4.1.9 | 4.1.10 | backend / frontend |
| vite | 8.0.16 | 8.1.4 | frontend |
| tsx | 4.22.4 | 4.23.0 | backend |
| prettier | 3.8.4 | 3.9.5 | backend |
| @types/node | 26.0.0 | 26.1.1 | backend / frontend |
| react-router-dom | 7.18.0 | 7.18.1 | frontend |
| tailwindcss / @tailwindcss/vite | 4.3.1 | 4.3.2 | frontend |
| @vitejs/plugin-react | 6.0.2 | 6.0.3 | frontend |
| globals | 17.6.0 | 17.7.0 | frontend |
| @playwright/test | 1.61.0 | 1.61.1 | e2e |

差分なし（すでに最新）: prisma / @prisma/client / @prisma/adapter-better-sqlite3（7.8.0）、express（5.2.1）、express-openapi-validator（5.6.2）、express-rate-limit（8.5.2）、zod（4.4.3）、@asteasolutions/zod-to-openapi（8.5.0）、bcrypt（6.0.0）、jsonwebtoken（9.0.3）、swagger-ui-express（5.0.1）、react / react-dom（19.2.7）、jsdom（29.1.1）、testing-library 系、madge（8.0.0）、supertest（7.2.2）。

#### メジャー更新（要個別対応）

**dependency-cruiser 17.4.3 → 18.0.0** — 今回実施

- Node 20 / 25 サポート削除 → Node 24 環境なので問題なし
- `tsconfig-paths-webpack-plugin` を enhanced-resolve 内蔵の tsconfig-paths 機能に置換 → backend の `@/` `@shared/` 等 path alias 解決の挙動確認が必須（`d review` の層依存・循環依存チェックが正しく動くこと）
- 参考: https://github.com/sverweij/dependency-cruiser/releases

**TypeScript 6.0.3 → 7.0.2 — 今回は見送り（決定済み）**

- TS7（tsgo、Go 製ネイティブコンパイラ）は Compiler API（Strada API）未提供。**typescript-eslint が動作しない**（公式も 7.1 待ちを表明: https://github.com/typescript-eslint/typescript-eslint/issues/10940）
- 本プロジェクトは lefthook pre-commit / CI / AI hook（lint-guard）で typescript-eslint を多用しており影響大
- その他の破壊的変更: `baseUrl` 削除、`moduleResolution: node10` 廃止、strict デフォルト ON、`types` デフォルト `[]` 等
- **再評価条件: TS 7.1 リリース + typescript-eslint の TS7 対応版リリース後**。それまでは TS 6.x 系の最新に追随
- 参考: https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/

## 移行計画

### Phase 1: Node engines の整合（低リスク）

実行環境はすでに Node 24 のため、宣言の修正のみ。

1. ルート / `backend/` / `frontend/` / `e2e/` の `package.json` engines を `">=24.0.0"` に変更
2. AGENTS.md 等に Node バージョン記載があれば整合確認（現状 docs には明示記載なし）

変更ファイル: `package.json`, `backend/package.json`, `frontend/package.json`, `e2e/package.json`

### Phase 2: minor / patch 一括更新（低リスク）

すべて caret / tilde 範囲内のため lockfile 更新が中心。

1. コンテナ内で `npm update`（ルートで実行、全 workspace 反映）
2. `package.json` の記載バージョンも最新に引き上げ（例: `eslint ^10.6.0`）— 任意だが推奨（lockfile と宣言の乖離防止）
3. 検証（下記「検証手順」参照）

注意点:

- prettier 3.8 → 3.9 で整形結果が変わる可能性 → `npm run format:check` で差分確認し、差分が出たら整形コミットを分離
- vite 8.0 → 8.1 は minor だが dev server / build の動作確認（`d dev` / `d bash -c "cd frontend && npm run build"`）

### Phase 3: dependency-cruiser 18 更新（中リスク）

1. `backend/package.json` の dependency-cruiser を `^18.0.0` に変更し `npm install`
2. `d review` を実行し、層依存・循環依存チェックが**更新前と同じ検出結果**になることを確認（path alias 解決方式が変わるため）
   - 更新前に `d review` の出力を保存しておき、更新後と比較する
3. 挙動が変わる場合は `.dependency-cruiser.cjs`（等の設定ファイル）の tsConfig / enhanced-resolve 設定を調整

### 見送り（今回対象外）

| 項目 | 理由 | 再評価タイミング |
|---|---|---|
| TypeScript 7 | typescript-eslint 非対応（Compiler API 未提供） | TS 7.1 + typescript-eslint 対応版リリース後 |
| Node 26 | Current であり LTS 未到達 | 2026-10-28 の LTS 化後 |
| Prisma 8 / Zod 5 / React 20 / Vite 9 / Tailwind 5 | 未リリース | リリース後 |

## 検証手順（各 Phase 完了ごと）

```bash
d lint                # ESLint (backend + frontend)
d typecheck           # tsc --noEmit (backend + frontend)
d test                # backend + frontend テスト
d review --strict     # 層依存・循環依存・複雑度・型・カバレッジ 80%
d e2e                 # Playwright E2E（Phase 2, 3 後）
d bash -c "cd backend && npm run format:check"   # prettier 差分確認（Phase 2 後）
d bash -c "cd frontend && npm run build"          # 本番ビルド確認（Phase 2 後）
```

すべて通過後、Phase ごとに個別コミット（日本語 Conventional Commits）。

## ロールバック

- 各 Phase を独立コミットにするため、問題発生時は該当コミットを `git revert`
- lockfile と `package.json` を同一コミットで戻し、コンテナ内で `npm install` を再実行

## 参考 URL

- Node.js リリーススケジュール: https://github.com/nodejs/release / https://endoflife.date/nodejs
- TypeScript 7.0 アナウンス: https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/
- typescript-eslint TS7 対応状況: https://github.com/typescript-eslint/typescript-eslint/issues/10940
- dependency-cruiser v18: https://github.com/sverweij/dependency-cruiser/releases
- Prisma 今後の方針（"Prisma Next"）: https://www.prisma.io/blog/the-next-evolution-of-prisma-orm
- Zod v4 changelog: https://zod.dev/v4/changelog
