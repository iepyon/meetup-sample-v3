use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_params_map};
use wasm_bindgen::JsValue;

use crate::api;
use crate::components::{Button, Card, ErrorAlert, Input};
use crate::types::{CreateEventRequest, EventFormat};
use crate::utils::{
    EventFormInput, api_error_message, format_label, parse_capacity, validate_event_form,
};

/// `datetime-local` の値（ローカル時刻）を Date にする。未入力・不正なら None。
fn parse_local(value: &str) -> Option<js_sys::Date> {
    if value.is_empty() {
        return None;
    }
    let date = js_sys::Date::new(&JsValue::from_str(value));
    (!date.get_time().is_nan()).then_some(date)
}

#[component]
pub fn EventCreatePage() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let title = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let starts_at = RwSignal::new(String::new());
    let ends_at = RwSignal::new(String::new());
    let format = RwSignal::new(EventFormat::Online);
    let capacity = RwSignal::new("50".to_string());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        let Some(community_id) = params.read_untracked().get("id") else {
            return;
        };
        let starts = parse_local(&starts_at.get_untracked());
        let ends = parse_local(&ends_at.get_untracked());
        let title_value = title.get_untracked();
        let description_value = description.get_untracked();
        let capacity_value = parse_capacity(&capacity.get_untracked());
        let checked = validate_event_form(&EventFormInput {
            title: &title_value,
            description: &description_value,
            starts_at_ms: starts.as_ref().map(js_sys::Date::get_time),
            ends_at_ms: ends.as_ref().map(js_sys::Date::get_time),
            capacity: capacity_value,
            now_ms: js_sys::Date::now(),
        });
        if let Err(message) = checked {
            error.set(Some(message.into()));
            return;
        }
        let (Some(starts), Some(ends)) = (starts, ends) else {
            return;
        };
        let (starts, ends) = (
            String::from(starts.to_iso_string()),
            String::from(ends.to_iso_string()),
        );
        loading.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            let request = CreateEventRequest {
                title: title_value,
                description: description_value,
                starts_at: starts,
                ends_at: ends,
                format: format.get_untracked(),
                capacity: capacity_value,
            };
            let path = format!("/communities/{community_id}/events");
            match api::post::<serde_json::Value>(&path, &request).await {
                Ok(_) => navigate(&format!("/communities/{community_id}"), Default::default()),
                Err(e) => error.set(Some(api_error_message(
                    e.status,
                    &e.message,
                    "イベントの作成に失敗しました",
                ))),
            }
            loading.set(false);
        });
    };

    view! {
        <div class="flex justify-center">
            <Card class="w-full max-w-lg">
                <h1 class="mb-6 text-2xl font-bold">"イベント作成"</h1>
                <ErrorAlert message=error />
                <form on:submit=on_submit>
                    <Input label="タイトル" value=title />
                    <div class="mb-4">
                        <label
                            for="event-description"
                            class="block text-sm font-medium text-gray-700 mb-1"
                        >
                            "説明"
                        </label>
                        <textarea
                            id="event-description"
                            class="w-full rounded border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                            rows="4"
                            prop:value=move || description.get()
                            on:input=move |ev| description.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                    <Input label="開始日時" kind="datetime-local" value=starts_at />
                    <Input label="終了日時" kind="datetime-local" value=ends_at />
                    <div class="mb-4">
                        <label
                            for="event-format"
                            class="block text-sm font-medium text-gray-700 mb-1"
                        >
                            "開催形式"
                        </label>
                        <select
                            id="event-format"
                            class="w-full rounded border border-gray-300 px-3 py-2 text-sm"
                            on:change=move |ev| {
                                if let Some(f) = EventFormat::parse(&event_target_value(&ev)) {
                                    format.set(f);
                                }
                            }
                        >
                            {EventFormat::ALL
                                .iter()
                                .map(|&f| {
                                    view! {
                                        <option value=f.as_str() selected=move || format.get() == f>
                                            {format_label(f)}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </div>
                    <Input label="定員" kind="number" min="1" max="1000" value=capacity />
                    <Button kind="submit" loading=loading class="w-full">
                        "作成"
                    </Button>
                </form>
            </Card>
        </div>
    }
}
