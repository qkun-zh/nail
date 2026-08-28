use leptos::prelude::*;

use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::mark_session_invalid;

#[component]
pub fn Logout() -> impl IntoView {
    let notifications = use_notifications();
    let working = RwSignal::new(false);

    let logout = move || {
        if working.get() {
            return;
        }
        working.set(true);
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::auth::delete_session().await;
            match result {
                Ok(_) => {
                    crate::request::session::clear_session_token();
                    mark_session_invalid();
                    notify_success(&notifications, "logged out");
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            working.set(false);
        });
    };

    view! {
        <div class="panel-page">
            <div class="panel-frame">
                <div class="panel-inner">
                    <h1 class="panel-title">"LOGOUT"</h1>
                    <div class="panel-form panel-form--center">
                        <button class="panel-submit" on:click=move |_| logout() disabled=move || working.get()>
                            {move || if working.get() { "logout..." } else { "logout" }}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
