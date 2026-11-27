use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::auth::{Auth, login, register};
use crate::components::{Button, Card, ErrorAlert, Input};

#[component]
pub fn LoginPage() -> impl IntoView {
    let auth = expect_context::<Auth>();
    let navigate = use_navigate();
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        if email.get().is_empty() || password.get().is_empty() {
            error.set(Some("メールアドレスとパスワードを入力してください".into()));
            return;
        }
        loading.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            match login(auth, &email.get_untracked(), &password.get_untracked()).await {
                Ok(()) => navigate("/", Default::default()),
                Err(message) => error.set(Some(message)),
            }
            loading.set(false);
        });
    };

    view! {
        <div class="flex min-h-[60vh] items-center justify-center">
            <Card class="w-full max-w-md">
                <h1 class="mb-6 text-2xl font-bold">"ログイン"</h1>
                <ErrorAlert message=error />
                <form on:submit=on_submit>
                    <Input label="メールアドレス" kind="email" value=email />
                    <Input label="パスワード" kind="password" value=password />
                    <Button kind="submit" loading=loading class="w-full">
                        "ログイン"
                    </Button>
                </form>
                <p class="mt-4 text-center text-sm text-gray-600">
                    "アカウントをお持ちでない方は "
                    <A href="/register" attr:class="text-blue-600 hover:underline">
                        "新規登録"
                    </A>
                </p>
            </Card>
        </div>
    }
}

#[component]
pub fn RegisterPage() -> impl IntoView {
    let auth = expect_context::<Auth>();
    let navigate = use_navigate();
    let name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        if name.get().is_empty() || email.get().is_empty() || password.get().is_empty() {
            error.set(Some("すべてのフィールドを入力してください".into()));
            return;
        }
        loading.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            let result = register(
                auth,
                &name.get_untracked(),
                &email.get_untracked(),
                &password.get_untracked(),
            )
            .await;
            match result {
                Ok(()) => navigate("/", Default::default()),
                Err(message) => error.set(Some(message)),
            }
            loading.set(false);
        });
    };

    view! {
        <div class="flex min-h-[60vh] items-center justify-center">
            <Card class="w-full max-w-md">
                <h1 class="mb-6 text-2xl font-bold">"新規登録"</h1>
                <ErrorAlert message=error />
                <form on:submit=on_submit>
                    <Input label="名前" value=name />
                    <Input label="メールアドレス" kind="email" value=email />
                    <Input label="パスワード" kind="password" value=password />
                    <Button kind="submit" loading=loading class="w-full">
                        "登録"
                    </Button>
                </form>
                <p class="mt-4 text-center text-sm text-gray-600">
                    "すでにアカウントをお持ちの方は "
                    <A href="/login" attr:class="text-blue-600 hover:underline">
                        "ログイン"
                    </A>
                </p>
            </Card>
        </div>
    }
}
