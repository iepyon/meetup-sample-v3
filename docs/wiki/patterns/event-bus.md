---
type: pattern
title: Event Bus
description: コンテキスト間連携をドメインイベントで疎結合化する EventBus<E>（現状は publish のみ）
tags: [domain-events, core-pattern]
sources:
  - backend/src/shared/event_bus.rs
  - backend/src/app.rs
  - backend/src/community/errors.rs
  - backend/src/event/errors.rs
  - docs/decision-logs/arch-20260419-in-memory-event-bus.md
  - docs/decision-logs/arch-20261127-rewrite-in-rust.md
verified: 5891e84
---

# Event Bus

モジュラーモノリス内のコンテキスト間連携をドメインイベントで疎結合化する。なぜインメモリかは [decision-log](../../decision-logs/arch-20260419-in-memory-event-bus.md) 参照。

## 実装（backend/src/shared/event_bus.rs）

- `EventBus<E>` — `RwLock<Vec<Box<dyn Fn(&E) + Send + Sync>>>` を持つジェネリック struct（`E` に trait 制約なし。`Default` で空の bus を作る）
  - `subscribe(handler)` — **イベント種別での絞り込みは無く**、全イベントが全ハンドラに渡る。ハンドラは同期の `Fn(&E)`
  - `publish(&event)` — 登録ハンドラを登録順に同期呼び出し

## 使用箇所

- 生成は `backend/src/app.rs` の `AppState::new`: `community_events: Arc<EventBus<CommunityDomainEvent>>` と `event_events: Arc<EventBus<EventCreatedEvent>>`。controller が `Ctx.bus` として UseCase に渡す
- 発行は UseCase 内。お手本: `community/usecases.rs` の `create_community_command`（保存後に `CommunityDomainEvent::CommunityCreated { … }` を `publish`）
- イベント型は `errors.rs` に定義（`community/errors.rs` の `CommunityDomainEvent` enum = CommunityCreated / MemberJoined 等、`event/errors.rs` の `EventCreatedEvent` struct）。発火時刻 `occurred_at` は controller が渡した時刻を使う

## 現状の重要な留意点

**`subscribe` による購読ハンドラの登録は現状どこにも無い**。publish 側の器だけが用意され、bus は 2 つ（community 用・event 用）が別インスタンスで共有もされていない。「発行の器は用意済み、購読側の実装は今後」が正確な状態。クロスコンテキストの副作用を追加するときは、bus の共有方法（`AppState` 内の bus の構成）から設計が必要。なおハンドラは同期実行なので、重い処理や async 処理を載せるには設計の見直しが要る。

## ルール

- イベントは enum（または struct）で、コンテキストの DomainEvent 型を bus の型引数にする
- UseCase は永続化に成功してから `publish` する（現状の全 command が保存後に発行）
