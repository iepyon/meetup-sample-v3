use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::api;
use crate::components::{Card, CategoryBadge, VisibilityBadge};
use crate::types::{Category, Community, ListCommunitiesResponse};
use crate::utils::{category_label, filter_by_category};

/// コミュニティカード（一覧・マイコミュニティ共通）
#[component]
pub fn CommunityCard(
    community: Community,
    #[prop(default = false)] show_badges: bool,
) -> impl IntoView {
    let href = format!("/communities/{}", community.id);
    let description = community.description.clone().unwrap_or_default();
    view! {
        <A href=href>
            <Card class="hover:shadow-md transition-shadow">
                <h2 class="text-lg font-semibold">{community.name.clone()}</h2>
                <p class="mt-1 text-sm text-gray-600">{description}</p>
                {show_badges
                    .then(|| {
                        view! {
                            <div class="mt-3 flex gap-2">
                                <CategoryBadge category=community.category />
                                <VisibilityBadge visibility=community.visibility />
                            </div>
                        }
                    })}
            </Card>
        </A>
    }
}

/// コミュニティ一覧を取得して状態シグナルに反映する（`path` は `/communities` 系）
pub fn load_communities(
    path: &'static str,
    error_message: &'static str,
    communities: RwSignal<Vec<Community>>,
    loading: RwSignal<bool>,
    error: RwSignal<Option<String>>,
) {
    loading.set(true);
    error.set(None);
    spawn_local(async move {
        match api::get::<ListCommunitiesResponse>(path).await {
            Ok(res) => communities.set(res.communities),
            Err(_) => error.set(Some(error_message.to_string())),
        }
        loading.set(false);
    });
}

#[component]
pub fn CommunityListPage() -> impl IntoView {
    let communities = RwSignal::new(Vec::<Community>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let filter = RwSignal::new(None::<Category>);

    Effect::new(move |_| {
        load_communities(
            "/communities",
            "コミュニティの取得に失敗しました",
            communities,
            loading,
            error,
        );
    });

    let filtered = move || filter_by_category(&communities.get(), filter.get());

    view! {
        <div>
            <div class="mb-6 flex items-center justify-between">
                <h1 class="text-2xl font-bold">"コミュニティ一覧"</h1>
                <select
                    class="rounded border border-gray-300 px-3 py-2 text-sm"
                    aria-label="カテゴリフィルター"
                    on:change=move |ev| filter.set(Category::parse(&event_target_value(&ev)))
                >
                    <option value="" selected=move || filter.get().is_none()>
                        "すべてのカテゴリ"
                    </option>
                    {Category::ALL
                        .iter()
                        .map(|&cat| {
                            view! {
                                <option
                                    value=cat.as_str()
                                    selected=move || filter.get() == Some(cat)
                                >
                                    {category_label(cat)}
                                </option>
                            }
                        })
                        .collect_view()}
                </select>
            </div>

            <Show when=move || loading.get()>
                <p>"読み込み中..."</p>
            </Show>
            {move || error.get().map(|e| view! { <p class="text-red-600">{e}</p> })}

            <Show when=move || !loading.get() && filtered().is_empty()>
                <p class="text-gray-500">"コミュニティが見つかりません"</p>
            </Show>

            <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                <For
                    each=filtered
                    key=|c| c.id.clone()
                    children=|community| {
                        view! { <CommunityCard community show_badges=true /> }
                    }
                />
            </div>
        </div>
    }
}
