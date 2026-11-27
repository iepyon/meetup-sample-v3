//! Meetup コミュニティ管理 API（Rust 実装）
//!
//! DDD の境界づけられたコンテキストごとにモジュールを分け、
//! `controller → usecases → models / repository` の依存方向を保つ。

pub mod app;
pub mod auth;
pub mod community;
pub mod event;
pub mod infrastructure;
pub mod shared;

pub use app::{AppState, create_app};
