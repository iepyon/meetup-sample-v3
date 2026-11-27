use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::api;
use crate::auth::Auth;
use crate::components::{Button, Card, CategoryBadge, Variant, VisibilityBadge};
use crate::types::{
    CommunityEvent, CommunityMember, CommunityResponse, ListEventsResponse, ListMembersResponse,
    MemberStatus, Role,
};
use crate::utils::is_uuid;

/// メンバーカード（OWNER には承認待ち（PENDING）の申請者にだけ承認/拒否ボタンを出す）
#[component]
fn MemberCard(
    member: CommunityMember,
    is_owner: Memo<bool>,
    on_approve: Callback<String>,
    on_reject: Callback<String>,
) -> impl IntoView {
    let display = member
        .account_name
        .clone()
        .or_else(|| member.account_id.clone())
        .unwrap_or_default();
    let target_is_pending = member.status == MemberStatus::Pending;
    let member_id = member.id.clone();
    view! {
        <Card class="flex items-center justify-between">
            <div>
                <p class="font-medium">{display}</p>
                <p class="text-sm text-gray-500">
                    {member.role.as_str()} " / " {member.status.as_str()}
                </p>
            </div>
            <Show when=move || is_owner.get() && target_is_pending>
                <div class="flex gap-2">
                    <Button on_click=Callback::new({
                        let id = member_id.clone();
                        move |_| on_approve.run(id.clone())
                    })>"承認"</Button>
                    <Button
                        variant=Variant::Danger
                        on_click=Callback::new({
                            let id = member_id.clone();
                            move |_| on_reject.run(id.clone())
                        })
                    >
                        "拒否"
                    </Button>
                </div>
            </Show>
        </Card>
    }
}

#[component]
pub fn CommunityDetailPage() -> impl IntoView {
    let auth = expect_context::<Auth>();
    let params = use_params_map();
    let navigate = use_navigate();

    let community = RwSignal::new(None::<crate::types::Community>);
    let community_loading = RwSignal::new(true);
    let community_error = RwSignal::new(None::<String>);
    let members = RwSignal::new(Vec::<CommunityMember>::new());
    let members_loading = RwSignal::new(false);
    let members_error = RwSignal::new(None::<String>);
    let events = RwSignal::new(Vec::<CommunityEvent>::new());
    let events_error = RwSignal::new(None::<String>);
    // :id が連続で変わったとき、古い id のレスポンスで状態を上書きしないための世代番号
    let generation = RwSignal::new(0_u32);

    // UUID 以外の :id は API パスに埋め込まない（空文字として「見つからない」扱いにする）
    let id = move || {
        params
            .read()
            .get("id")
            .filter(|v| is_uuid(v))
            .unwrap_or_default()
    };

    let load_members = move |id: String| {
        members_loading.set(true);
        members_error.set(None);
        spawn_local(async move {
            match api::get::<ListMembersResponse>(&format!("/communities/{id}/members")).await {
                Ok(res) => members.set(res.members),
                Err(_) => members_error.set(Some("メンバー一覧の取得に失敗しました".into())),
            }
            members_loading.set(false);
        });
    };

    Effect::new(move |_| {
        let id = id();
        // 世代を進めて、前の id の応答が後から届いても状態を上書きさせない
        generation.update_untracked(|g| *g += 1);
        if id.is_empty() {
            community.set(None);
            members.set(Vec::new());
            events.set(Vec::new());
            community_loading.set(false);
            return;
        }
        let current = generation.get_untracked();
        community_loading.set(true);
        community_error.set(None);
        events_error.set(None);
        spawn_local({
            let id = id.clone();
            async move {
                let result = api::get::<CommunityResponse>(&format!("/communities/{id}")).await;
                if generation.get_untracked() != current {
                    return;
                }
                match result {
                    Ok(res) => community.set(Some(res.community)),
                    Err(_) => {
                        community.set(None);
                        community_error.set(Some("コミュニティの取得に失敗しました".into()));
                    }
                }
                community_loading.set(false);
            }
        });
        load_members(id.clone());
        spawn_local(async move {
            let result = api::get::<ListEventsResponse>(&format!("/communities/{id}/events")).await;
            if generation.get_untracked() != current {
                return;
            }
            match result {
                Ok(res) => events.set(res.events),
                Err(_) => events_error.set(Some("イベント一覧の取得に失敗しました".into())),
            }
        });
    });

    // 自分のメンバー情報（accountId は ACTIVE メンバー本人にだけ返る）
    let me = Memo::new(move |_| {
        let user_id = auth.user.get().map(|u| u.id)?;
        members
            .read()
            .iter()
            .find(|m| m.account_id.as_deref() == Some(user_id.as_str()))
            .cloned()
    });
    let is_owner = Memo::new(move |_| me.get().is_some_and(|m| m.role == Role::Owner));
    let can_manage_events = Memo::new(move |_| {
        me.get()
            .is_some_and(|m| matches!(m.role, Role::Owner | Role::Admin))
    });

    let on_join = move |_| {
        let id = id();
        spawn_local(async move {
            match api::post_empty::<serde_json::Value>(&format!("/communities/{id}/members")).await
            {
                Ok(_) => load_members(id),
                Err(_) => members_error.set(Some("コミュニティへの参加に失敗しました".into())),
            }
        });
    };
    let on_leave = move |_| {
        let (id, Some(member)) = (id(), me.get()) else {
            return;
        };
        spawn_local(async move {
            let path = format!("/communities/{id}/members/{}", member.id);
            match api::delete::<serde_json::Value>(&path).await {
                Ok(_) => load_members(id),
                Err(_) => members_error.set(Some("コミュニティからの退会に失敗しました".into())),
            }
        });
    };
    let on_approve = Callback::new(move |member_id: String| {
        let id = id();
        spawn_local(async move {
            let path = format!("/communities/{id}/members/{member_id}/approve");
            match api::patch::<serde_json::Value>(&path).await {
                Ok(_) => load_members(id),
                Err(_) => members_error.set(Some("メンバーの承認に失敗しました".into())),
            }
        });
    });
    let on_reject = Callback::new(move |member_id: String| {
        let id = id();
        spawn_local(async move {
            let path = format!("/communities/{id}/members/{member_id}/reject");
            match api::patch::<serde_json::Value>(&path).await {
                Ok(_) => load_members(id),
                Err(_) => members_error.set(Some("メンバーの拒否に失敗しました".into())),
            }
        });
    });

    view! {
        {move || {
            if community_loading.get() {
                return view! { <p>"読み込み中..."</p> }.into_any();
            }
            if let Some(message) = community_error.get() {
                return view! { <p class="text-red-600">{message}</p> }.into_any();
            }
            let Some(c) = community.get() else {
                return view! { <p>"コミュニティが見つかりません"</p> }.into_any();
            };
            let navigate = navigate.clone();
            view! {
                <div>
                    <Card class="mb-6">
                        <h1 class="text-2xl font-bold">{c.name.clone()}</h1>
                        <p class="mt-2 text-gray-600">{c.description.clone().unwrap_or_default()}</p>
                        <div class="mt-4 flex gap-2">
                            <CategoryBadge category=c.category />
                            <VisibilityBadge visibility=c.visibility />
                        </div>

                        <Show when=move || {
                            auth.user.get().is_some() && !members_loading.get() && me.get().is_none()
                        }>
                            <Button on_click=Callback::new(on_join) class="mt-4">
                                "参加する"
                            </Button>
                        </Show>
                        <Show when=move || me.get().is_some() && !is_owner.get()>
                            <Button
                                on_click=Callback::new(on_leave)
                                variant=Variant::Danger
                                class="mt-4"
                            >
                                "退会する"
                            </Button>
                        </Show>
                        <Show when=move || can_manage_events.get()>
                            <Button
                                on_click=Callback::new({
                                    let navigate = navigate.clone();
                                    move |_| navigate(
                                        &format!("/communities/{}/events/new", id()),
                                        Default::default(),
                                    )
                                })
                                class="mt-4 ml-2"
                            >
                                "イベント作成"
                            </Button>
                        </Show>
                    </Card>

                    <h2 class="mb-4 text-xl font-semibold">"イベント一覧"</h2>
                    {move || events_error.get().map(|e| view! { <p class="text-red-600">{e}</p> })}
                    <div class="mb-6 space-y-2">
                        <Show when=move || events.get().is_empty()>
                            <p class="text-gray-500">"イベントはありません"</p>
                        </Show>
                        <For
                            each=move || events.get()
                            key=|e| e.id.clone()
                            children=|ev| {
                                view! {
                                    <Card class="flex items-center justify-between">
                                        <span class="font-medium text-gray-900">{ev.title}</span>
                                        <span class="text-sm text-gray-500">{ev.status.as_str()}</span>
                                    </Card>
                                }
                            }
                        />
                    </div>

                    <h2 class="mb-4 text-xl font-semibold">"メンバー一覧"</h2>
                    <Show when=move || members_loading.get()>
                        <p>"読み込み中..."</p>
                    </Show>
                    {move || {
                        members_error.get().map(|e| view! { <p class="text-red-600">{e}</p> })
                    }}
                    <div class="space-y-3">
                        <For
                            each=move || members.get()
                            key=|m| (m.id.clone(), m.status)
                            children=move |member| {
                                view! {
                                    <MemberCard
                                        member
                                        is_owner
                                        on_approve
                                        on_reject
                                    />
                                }
                            }
                        />
                    </div>
                </div>
            }
                .into_any()
        }}
    }
}
