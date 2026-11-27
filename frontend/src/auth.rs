//! 認証コンテキスト（ログイン状態の保持・ログイン/登録/ログアウト）

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;

use crate::api;
use crate::token::{get_token, remove_token, set_token};
use crate::types::{Account, AccountResponse, AuthResponse};
use crate::utils::api_error_message;

#[derive(Clone, Copy)]
pub struct Auth {
    pub user: RwSignal<Option<Account>>,
    /// 保存済みトークンでユーザー情報を取得している間は true
    pub loading: RwSignal<bool>,
}

/// コンテキストを提供し、保存済みトークンがあれば `/auth/me` でユーザーを復元する。
pub fn provide_auth() -> Auth {
    let auth = Auth {
        user: RwSignal::new(None),
        loading: RwSignal::new(get_token().is_some()),
    };
    provide_context(auth);

    if get_token().is_some() {
        spawn_local(async move {
            if let Ok(res) = api::get::<AccountResponse>("/auth/me").await {
                auth.user.set(Some(res.account));
            }
            auth.loading.set(false);
        });
    }
    auth
}

#[derive(Serialize)]
struct LoginRequest<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
struct RegisterRequest<'a> {
    name: &'a str,
    email: &'a str,
    password: &'a str,
}

fn accept(auth: Auth, res: AuthResponse) {
    set_token(&res.token);
    auth.user.set(Some(res.account));
}

pub async fn login(auth: Auth, email: &str, password: &str) -> Result<(), String> {
    api::post::<AuthResponse>("/auth/login", &LoginRequest { email, password })
        .await
        .map(|res| accept(auth, res))
        .map_err(|_| "ログインに失敗しました".to_string())
}

pub async fn register(auth: Auth, name: &str, email: &str, password: &str) -> Result<(), String> {
    api::post::<AuthResponse>(
        "/auth/register",
        &RegisterRequest {
            name,
            email,
            password,
        },
    )
    .await
    .map(|res| accept(auth, res))
    .map_err(|e| api_error_message(e.status, &e.message, "登録に失敗しました"))
}

pub fn logout(auth: Auth) {
    remove_token();
    auth.user.set(None);
}
