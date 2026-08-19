use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::validate_uuid;

fn split_entries(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

#[component]
pub fn UpdateRole() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let permissions_add = RwSignal::new(String::new());
    let permissions_remove = RwSignal::new(String::new());
    let users_add = RwSignal::new(String::new());
    let users_remove = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if submitting.get() {
            return;
        }
        submitting.set(true);
        error.set(None);

        let role_id = params.get().get("role_id").unwrap_or_default();
        if let Err(message) = validate_uuid(&role_id) {
            notify_error(&notifications, message.clone());
            error.set(Some(message));
            submitting.set(false);
            return;
        }
        let permissions_add = split_entries(&permissions_add.get());
        let permissions_remove = split_entries(&permissions_remove.get());
        let users_add = split_entries(&users_add.get());
        let users_remove = split_entries(&users_remove.get());
        let navigate = navigate.clone();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::role::update_role(
                &role_id,
                &permissions_add,
                &permissions_remove,
                &users_add,
                &users_remove,
            )
            .await;
            submitting.set(false);
            match result {
                Ok(_) => {
                    notify_success(&notifications, "role updated");
                    navigate(&format!("/role/{role_id}"), NavigateOptions::default());
                }
                Err(err) => {
                    notify_error(&notifications, err.to_string());
                    error.set(Some(err.to_string()));
                }
            }
        });
    };

    view! {
        <h1>"Update Role"</h1>
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <form on:submit=on_submit>
            <label>
                "Permissions to add (comma separated)"
                <input
                    type="text"
                    prop:value=permissions_add
                    on:input=move |ev| permissions_add.set(event_target_value(&ev))
                    disabled=submitting
                />
            </label>
            <label>
                "Permissions to remove (comma separated)"
                <input
                    type="text"
                    prop:value=permissions_remove
                    on:input=move |ev| permissions_remove.set(event_target_value(&ev))
                    disabled=submitting
                />
            </label>
            <label>
                "User ids to add (comma separated)"
                <input
                    type="text"
                    prop:value=users_add
                    on:input=move |ev| users_add.set(event_target_value(&ev))
                    disabled=submitting
                />
            </label>
            <label>
                "User ids to remove (comma separated)"
                <input
                    type="text"
                    prop:value=users_remove
                    on:input=move |ev| users_remove.set(event_target_value(&ev))
                    disabled=submitting
                />
            </label>
            <button type="submit" disabled=submitting>
                {move || if submitting.get() { "Updating..." } else { "Update" }}
            </button>
        </form>
    }
}
