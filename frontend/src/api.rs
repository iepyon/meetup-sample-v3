//! API クライアント（fetch ラッパー）。失敗は `ApiFailure` として値で返す。

use gloo_net::http::{Method, RequestBuilder};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::token::{get_token, remove_token};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ApiFailure {
    pub status: u16,
    /// サーバーが返した `{ code, message }` の message（取得できなければ空）
    pub message: String,
}

fn failure(status: u16, message: impl Into<String>) -> ApiFailure {
    ApiFailure {
        status,
        message: message.into(),
    }
}

async fn request<T: DeserializeOwned>(
    method: Method,
    path: &str,
    body: Option<String>,
) -> Result<T, ApiFailure> {
    let mut builder = RequestBuilder::new(path)
        .method(method)
        .header("Content-Type", "application/json");
    let sent_token = get_token();
    if let Some(token) = &sent_token {
        builder = builder.header("Authorization", &format!("Bearer {token}"));
    }
    let request = match body {
        Some(b) => builder.body(b),
        None => builder.build(),
    }
    .map_err(|_| failure(0, "リクエストを作成できませんでした"))?;

    let response = request
        .send()
        .await
        .map_err(|_| failure(0, "ネットワークエラーが発生しました"))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    // 204 No Content などの空ボディは null として扱う
    let json = if text.trim().is_empty() {
        "null"
    } else {
        text.as_str()
    };

    // 期限切れ・無効なトークンを残さない（以後のリクエストに付け続けないように）。
    // 応答待ちの間に再ログインしていた場合は新しいトークンを消さない
    if status == 401 && sent_token.is_some() && sent_token == get_token() {
        remove_token();
    }

    if response.ok() {
        serde_json::from_str(json).map_err(|_| failure(status, "レスポンスを解釈できませんでした"))
    } else {
        let message = serde_json::from_str::<serde_json::Value>(json)
            .ok()
            .and_then(|v| {
                v.get("message")
                    .and_then(|m| m.as_str().map(str::to_string))
            })
            .unwrap_or_default();
        Err(failure(status, message))
    }
}

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, ApiFailure> {
    request(Method::GET, path, None).await
}

pub async fn post<T: DeserializeOwned>(path: &str, body: &impl Serialize) -> Result<T, ApiFailure> {
    let body = serde_json::to_string(body).map_err(|_| failure(0, "送信データが不正です"))?;
    request(Method::POST, path, Some(body)).await
}

/// ボディなしの POST（コミュニティ参加など）
pub async fn post_empty<T: DeserializeOwned>(path: &str) -> Result<T, ApiFailure> {
    request(Method::POST, path, None).await
}

pub async fn patch<T: DeserializeOwned>(path: &str) -> Result<T, ApiFailure> {
    request(Method::PATCH, path, None).await
}

pub async fn delete<T: DeserializeOwned>(path: &str) -> Result<T, ApiFailure> {
    request(Method::DELETE, path, None).await
}
