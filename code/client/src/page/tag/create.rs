use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::page::notify::{notify_error, use_notifications};
use crate::request::tag;

#[component]
pub fn CreateTag() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let navigate = use_navigate();
    let notifications = use_notifications();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);

        let name = name.get();
        let navigate = navigate.clone();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match tag::create_tag(&name).await {
                Ok(tag) => {
                    navigate(&format!("/tag/{}", tag.id), NavigateOptions::default());
                }
                Err(err) => {
                    notify_error(&notifications, err.to_string());
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Create Tag"</h1>
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
                {move || if submitting.get() { "Creating..." } else { "Create" }}
            </button>
        </form>
    }
}
