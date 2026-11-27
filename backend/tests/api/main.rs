//! backend の HTTP 統合テスト（インメモリ SQLite のアプリを `oneshot` で直接呼ぶ）。
//! テストのバイナリを 1 つにまとめ、観点ごとにモジュールを分ける。
//! 絞り込み: `cargo test -p meetup-backend --test api member::`

mod common;

mod auth;
mod community;
mod event;
mod http;
mod member;
mod member_list;
mod member_review;
mod rate_limit;
mod spa;
