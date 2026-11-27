//! 認証トークンの保存（localStorage の `auth_token`）

const TOKEN_KEY: &str = "auth_token";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn get_token() -> Option<String> {
    storage()?.get_item(TOKEN_KEY).ok().flatten()
}

pub fn set_token(token: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(TOKEN_KEY, token);
    }
}

pub fn remove_token() {
    if let Some(s) = storage() {
        let _ = s.remove_item(TOKEN_KEY);
    }
}
