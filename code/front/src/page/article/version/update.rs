use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::notify::{notify_error, notify_success, use_notifications};

#[component]
pub fn UpdateVersion() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let note = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if submitting.get() {
            return;
        }
        submitting.set(true);
        error.set(None);

        let version_id = params.get().get("version_id").unwrap_or_default();
        let article_id = params.get().get("article_id").unwrap_or_default();
        let note = note.get();
        let navigate = navigate.clone();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::version::update_version(&version_id, &note).await {
                Ok(_) => {
                    notify_success(&notifications, "version updated");
                    navigate(
                        &format!("/article/{article_id}/version/{version_id}"),
                        NavigateOptions::default(),
                    );
                }
                Err(err) => {
                    notify_error(&notifications, err.to_string());
                    error.set(Some(err.to_string()));
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Update Version"</h1>
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <form on:submit=on_submit>
            <label>
                "Note"
                <textarea
                    prop:value=note
                    on:input=move |ev| note.set(event_target_value(&ev))
                    disabled=submitting
                />
            </label>
            <button type="submit" disabled=submitting>
                {move || if submitting.get() { "Updating..." } else { "Update" }}
            </button>
        </form>
    }
}
