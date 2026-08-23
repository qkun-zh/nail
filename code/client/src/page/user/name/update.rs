use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

use crate::page::draft::mirror_text_param;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::{
    SessionStatus, authenticated_user_id, refresh_session, use_session_status,
};
use crate::page::validation::validate_name;

#[component]
pub fn NameUpdate() -> impl IntoView {
    let notifications = use_notifications();
    let query = use_query_map();
    let params = use_params_map();
    let status = use_session_status();
    let name = RwSignal::new(query.get_untracked().get("name").unwrap_or_default());
    let working = RwSignal::new(false);

    mirror_text_param("name", move || name.get());

    Effect::new(move |_| {
        let SessionStatus::Authenticated(view) = status.get() else {
            return;
        };
        if name.get_untracked().is_empty()
            && let Some(current) = view.name
            && !current.is_empty()
        {
            name.set(current);
        }
    });

    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let Some(user_id) = authenticated_user_id() else {
            notify_error(&notifications, "authenticate to rename");
            return;
        };
        if params
            .get_untracked()
            .get("uid")
            .is_some_and(|uid| uid != user_id)
        {
            notify_error(&notifications, "cannot rename another user");
            return;
        }
        let new_name = match validate_name(&name.get()) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };
        working.set(true);
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::user::update_self_name(&user_id, new_name).await;
            match result {
                Ok(_) => {
                    refresh_session();
                    notify_success(&notifications, "name updated");
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            working.set(false);
        });
    };

    view! {
            <div>
                <form on:submit=submit>
                    <input type="text" prop:value=name on:input=move |event| name.set(event_target_value(&event)) placeholder="name"/>
                    <button type="submit" disabled=move || working.get()>
                        {move || if working.get() { "updating..." } else { "update name" }}
                    </button>
                </form>
            </div>
    }
}
