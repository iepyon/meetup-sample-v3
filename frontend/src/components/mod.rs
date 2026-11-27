//! 共有 UI コンポーネント（Button / Input / Card / ErrorAlert / Layout / バッジ）

use leptos::prelude::*;
use leptos_router::components::A;

use crate::auth::{Auth, logout};
use crate::types::{Category, Visibility};
use crate::utils::{category_label, visibility_label};

#[component]
pub fn Card(#[prop(optional, into)] class: String, children: Children) -> impl IntoView {
    view! {
        <div class=format!("rounded-lg border border-gray-200 bg-white p-6 shadow-sm {class}")>
            {children()}
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    #[default]
    Primary,
    Danger,
}

#[component]
pub fn Button(
    #[prop(default = Variant::Primary)] variant: Variant,
    #[prop(optional, into)] loading: Signal<bool>,
    #[prop(optional, into)] class: String,
    #[prop(default = "button")] kind: &'static str,
    #[prop(optional, into)] on_click: Option<Callback<()>>,
    children: ChildrenFn,
) -> impl IntoView {
    let variant_class = match variant {
        Variant::Primary => "bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500",
        Variant::Danger => "bg-red-600 text-white hover:bg-red-700 focus:ring-red-500",
    };
    view! {
        <button
            type=kind
            class=format!(
                "rounded px-4 py-2 font-medium focus:outline-none focus:ring-2 disabled:opacity-50 disabled:cursor-not-allowed {variant_class} {class}",
            )
            disabled=move || loading.get()
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            {move || if loading.get() { "読み込み中...".into_any() } else { children().into_any() }}
        </button>
    }
}

#[component]
pub fn Input(
    label: &'static str,
    value: RwSignal<String>,
    #[prop(default = "text")] kind: &'static str,
    #[prop(optional, into)] min: Option<String>,
    #[prop(optional, into)] max: Option<String>,
) -> impl IntoView {
    view! {
        <div class="mb-4">
            <label for=label class="block text-sm font-medium text-gray-700 mb-1">
                {label}
            </label>
            <input
                id=label
                type=kind
                min=min
                max=max
                class="w-full rounded border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
        </div>
    }
}

#[component]
pub fn ErrorAlert(#[prop(into)] message: Signal<Option<String>>) -> impl IntoView {
    move || {
        message.get().filter(|m| !m.is_empty()).map(|m| {
            view! {
                <div class="mb-4 rounded bg-red-50 p-3 text-sm text-red-600" role="alert">
                    {m}
                </div>
            }
        })
    }
}

#[component]
pub fn CategoryBadge(category: Category) -> impl IntoView {
    view! {
        <span class="rounded-full bg-blue-100 px-2 py-1 text-xs text-blue-800">
            {category_label(category)}
        </span>
    }
}

#[component]
pub fn VisibilityBadge(visibility: Visibility) -> impl IntoView {
    view! {
        <span class="rounded-full bg-gray-100 px-2 py-1 text-xs text-gray-600">
            {visibility_label(visibility)}
        </span>
    }
}

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let auth = expect_context::<Auth>();
    let is_authenticated = move || auth.user.get().is_some();
    let user_name = move || auth.user.get().map(|u| u.name).unwrap_or_default();

    view! {
        <div class="min-h-screen bg-gray-50">
            <header class="bg-white shadow-sm">
                <div class="mx-auto max-w-7xl px-4 py-4 flex items-center justify-between">
                    <div class="flex items-center gap-6">
                        <A href="/" attr:class="text-xl font-bold text-blue-600">
                            "Meetup"
                        </A>
                        <nav class="flex gap-4">
                            <A href="/" attr:class="text-gray-600 hover:text-gray-900">
                                "コミュニティ一覧"
                            </A>
                            <Show when=is_authenticated>
                                <A
                                    href="/my-communities"
                                    attr:class="text-gray-600 hover:text-gray-900"
                                >
                                    "マイコミュニティ"
                                </A>
                                <A
                                    href="/communities/new"
                                    attr:class="text-gray-600 hover:text-gray-900"
                                >
                                    "コミュニティ作成"
                                </A>
                            </Show>
                        </nav>
                    </div>
                    <div class="flex items-center gap-4">
                        <Show
                            when=is_authenticated
                            fallback=|| {
                                view! {
                                    <A
                                        href="/login"
                                        attr:class="text-sm text-gray-600 hover:text-gray-900"
                                    >
                                        "ログイン"
                                    </A>
                                    <A
                                        href="/register"
                                        attr:class="text-sm text-blue-600 hover:text-blue-800"
                                    >
                                        "新規登録"
                                    </A>
                                }
                            }
                        >
                            <span class="text-sm text-gray-600">{user_name}</span>
                            <button
                                class="text-sm text-gray-600 hover:text-gray-900"
                                on:click=move |_| logout(auth)
                            >
                                "ログアウト"
                            </button>
                        </Show>
                    </div>
                </div>
            </header>
            <main class="mx-auto max-w-7xl px-4 py-8">{children()}</main>
        </div>
    }
}
