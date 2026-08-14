use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::{
    authenticated_user_id, refresh_session,
};
use crate::page::validation::validate_name;

#[component]
pub fn NameUpdate() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let name = RwSignal::new(String::new());
    let working = RwSignal::new(false);

    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let Some(user_id) = authenticated_user_id() else {
            notify_error(&notifications, "authenticate to rename");
            return;
        };
        let new_name = match validate_name(&name.get()) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };
        working.set(true);
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            let result = match crate::request::pow::prove_pow(new_name).await {
                Ok(pow) => crate::request::user::update_self_name(&user_id, pow).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(_) => {
                    refresh_session();
                    notify_success(&notifications, "name updated");
                    navigate(
                        "/private/name",
                        NavigateOptions {
                            resolve: false,
                            ..Default::default()
                        },
                    );
                }
                Err(error) => notify_error(&notifications, &error.to_string()),
            }
            working.set(false);
        });
    };

    view! {
            <div>
                <form on:submit=submit>
                    <input type="text" prop:value=name on:input=move |event| name.set(event_target_value(&event)) placeholder="new name"/>
                    <button type="submit" disabled=move || working.get()>save</button>
                </form>
            </div>
    }
}
