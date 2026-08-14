use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::mark_session_invalid;

#[component]
pub fn Logout() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let working = RwSignal::new(false);

    let logout = move || {
        if working.get() {
            return;
        }
        working.set(true);
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            let nonce = js_sys::Date::now().to_string();
            let result = match crate::request::pow::prove_pow(nonce).await {
                Ok(pow) => crate::request::auth::delete_session(pow).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(_) => {
                    crate::request::session::clear_session_token();
                    mark_session_invalid();
                    notify_success(&notifications, "logged out");
                    navigate(
                        "/",
                        NavigateOptions {
                            resolve: false,
                            ..Default::default()
                        },
                    );
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            working.set(false);
        });
    };

    view! {
            <div>
                <p>logout</p>
                <button on:click=move |_| logout() disabled=move || working.get()>log out</button>
            </div>
    }
}
