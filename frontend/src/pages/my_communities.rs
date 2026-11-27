use leptos::prelude::*;

use super::community_list::{CommunityCard, load_communities};
use crate::types::Community;

#[component]
pub fn MyCommunitiesPage() -> impl IntoView {
    let communities = RwSignal::new(Vec::<Community>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        load_communities(
            "/communities?member=me",
            "マイコミュニティの取得に失敗しました",
            communities,
            loading,
            error,
        );
    });

    view! {
        <div>
            <h1 class="mb-6 text-2xl font-bold">"マイコミュニティ"</h1>

            <Show when=move || loading.get()>
                <p>"読み込み中..."</p>
            </Show>
            {move || error.get().map(|e| view! { <p class="text-red-600">{e}</p> })}

            <Show when=move || !loading.get() && communities.get().is_empty()>
                <p class="text-gray-500">"参加しているコミュニティがありません"</p>
            </Show>

            <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                <For
                    each=move || communities.get()
                    key=|c| c.id.clone()
                    children=|community| view! { <CommunityCard community /> }
                />
            </div>
        </div>
    }
}
