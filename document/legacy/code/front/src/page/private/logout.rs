use crate::page::notify::{notify_error, notify_success, use_notify};
use crate::pow::prove;
use crate::req::{ProveInput, SESSION_TOKEN_KEY, get_challenge, post_logout};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;

fn short_nonce() -> String {
    let hi = (js_sys::Math::random() * (u32::MAX as f64)) as u32;
    let lo = (js_sys::Math::random() * (u32::MAX as f64)) as u32;
    format!("{hi:08x}{lo:08x}")
}

#[component]
pub fn Logout() -> impl IntoView {
    let notification = use_notify();
    let verified =
        use_context::<RwSignal<Option<bool>>>().unwrap_or_else(|| RwSignal::new(Some(false)));
    let logout_in_progress = RwSignal::new(false);

    let on_logout = move |_| {
        if logout_in_progress.get() {
            return;
        }
        let session_token = match LocalStorage::get::<String>(SESSION_TOKEN_KEY) {
            Ok(t) => t,
            Err(e) => {
                notify_error(&notification, &format!("failed to read session token: {e}"));
                return;
            }
        };
        if session_token.is_empty() {
            notify_error(&notification, "not authenticated");
            return;
        }
        logout_in_progress.set(true);
        spawn_local(async move {
            let challenge = match get_challenge().await {
                Ok(c) => c,
                Err(e) => {
                    notify_error(&notification, &format!("get challenge failed: {e}"));
                    logout_in_progress.set(false);
                    return;
                }
            };
            let pow = match prove(ProveInput {
                challenge,
                payload: short_nonce(),
            })
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    notify_error(&notification, &format!("proof failed: {e}"));
                    logout_in_progress.set(false);
                    return;
                }
            };
            match post_logout(&pow, &session_token).await {
                Ok(_) => {
                    LocalStorage::delete(SESSION_TOKEN_KEY);
                    verified.set(Some(false));
                    notify_success(&notification, "logout");
                }
                Err(e) => {
                    LocalStorage::delete(SESSION_TOKEN_KEY);
                    verified.set(Some(false));
                    notify_error(&notification, &format!("connection lost: {e}"));
                }
            }
            logout_in_progress.set(false);
        });
    };

    view! {
        <button class="logout-action" on:click=on_logout disabled=logout_in_progress>
            {move || if logout_in_progress.get() { "logout..." } else { "logout" }}
        </button>
    }
}
