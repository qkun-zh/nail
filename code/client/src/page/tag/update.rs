use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::notify::{notify_error, use_notifications};
use crate::page::validation::validate_uuid;
use crate::request::tag;

#[component]
pub fn UpdateTag() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let tag = RwSignal::new(None::<common::response::tag::TagListItem>);
    let name = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);

    let load_notifications = notifications.clone();
    Effect::new(move |_| {
        let tag_id = params.get().get("tag_id").unwrap_or_default();
        if let Err(message) = validate_uuid(&tag_id) {
            notify_error(&load_notifications, message);
            return;
        }
        let notifications = load_notifications.clone();
        leptos::task::spawn_local(async move {
            match tag::read_tag(&tag_id).await {
                Ok(tag_view) => {
                    name.set(tag_view.name.clone());
                    tag.set(Some(tag_view));
                }
                Err(err) => notify_error(&notifications, err.to_string()),
            }
        });
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);

        let tag_id = params.get().get("tag_id").unwrap_or_default();
        if let Err(message) = validate_uuid(&tag_id) {
            notify_error(&notifications, message);
            submitting.set(false);
            return;
        }
        let name = name.get();
        let navigate = navigate.clone();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match tag::update_tag(&tag_id, &name).await {
                Ok(_) => {
                    navigate(&format!("/tag/{tag_id}"), NavigateOptions::default());
                }
                Err(err) => {
                    notify_error(&notifications, err.to_string());
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Update Tag"</h1>
        <form on:submit=on_submit>
            <label>
                "Name"
                <input
                    type="text"
                    prop:value=name
                    on:input=move |ev| name.set(event_target_value(&ev))
                    disabled=submitting
                />
            </label>
            <button type="submit" disabled=submitting>
                {move || if submitting.get() { "Updating..." } else { "Update" }}
            </button>
        </form>
    }
}
