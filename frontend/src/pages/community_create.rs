use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::components::{Button, Card, ErrorAlert, Input};
use crate::types::{Category, CommunityResponse, CreateCommunityRequest, Visibility};
use crate::utils::category_label;

#[component]
pub fn CommunityCreatePage() -> impl IntoView {
    let navigate = use_navigate();
    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let category = RwSignal::new(Category::Tech);
    let visibility = RwSignal::new(Visibility::Public);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        if name.get().is_empty() || description.get().is_empty() {
            error.set(Some("名前と説明を入力してください".into()));
            return;
        }
        loading.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            let request = CreateCommunityRequest {
                name: name.get_untracked(),
                description: description.get_untracked(),
                category: category.get_untracked(),
                visibility: visibility.get_untracked(),
            };
            match api::post::<CommunityResponse>("/communities", &request).await {
                Ok(res) => navigate(
                    &format!("/communities/{}", res.community.id),
                    Default::default(),
                ),
                Err(_) => error.set(Some("コミュニティの作成に失敗しました".into())),
            }
            loading.set(false);
        });
    };

    view! {
        <div class="flex justify-center">
            <Card class="w-full max-w-lg">
                <h1 class="mb-6 text-2xl font-bold">"コミュニティ作成"</h1>
                <ErrorAlert message=error />
                <form on:submit=on_submit>
                    <Input label="コミュニティ名" value=name />
                    <div class="mb-4">
                        <label for="description" class="block text-sm font-medium text-gray-700 mb-1">
                            "説明"
                        </label>
                        <textarea
                            id="description"
                            class="w-full rounded border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                            rows="4"
                            prop:value=move || description.get()
                            on:input=move |ev| description.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                    <div class="mb-4">
                        <label for="category" class="block text-sm font-medium text-gray-700 mb-1">
                            "カテゴリ"
                        </label>
                        <select
                            id="category"
                            class="w-full rounded border border-gray-300 px-3 py-2 text-sm"
                            on:change=move |ev| {
                                if let Some(c) = Category::parse(&event_target_value(&ev)) {
                                    category.set(c);
                                }
                            }
                        >
                            {Category::ALL
                                .iter()
                                .map(|&cat| {
                                    view! {
                                        <option
                                            value=cat.as_str()
                                            selected=move || category.get() == cat
                                        >
                                            {category_label(cat)}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </div>
                    <div class="mb-6">
                        <label for="visibility" class="block text-sm font-medium text-gray-700 mb-1">
                            "公開設定"
                        </label>
                        <select
                            id="visibility"
                            class="w-full rounded border border-gray-300 px-3 py-2 text-sm"
                            on:change=move |ev| {
                                if let Some(v) = Visibility::parse(&event_target_value(&ev)) {
                                    visibility.set(v);
                                }
                            }
                        >
                            <option
                                value="PUBLIC"
                                selected=move || visibility.get() == Visibility::Public
                            >
                                "公開"
                            </option>
                            <option
                                value="PRIVATE"
                                selected=move || visibility.get() == Visibility::Private
                            >
                                "非公開"
                            </option>
                        </select>
                    </div>
                    <Button kind="submit" loading=loading class="w-full">
                        "作成"
                    </Button>
                </form>
            </Card>
        </div>
    }
}
