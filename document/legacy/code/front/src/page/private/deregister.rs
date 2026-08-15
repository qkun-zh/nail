use crate::page::notify::{notify_error, notify_success, use_notify};
use crate::pow::prove;
use crate::req::{
    ProveInput, SESSION_TOKEN_KEY, get_challenge, post_deregister_user,
    post_deregister_user_confirm,
};
use gloo_storage::{LocalStorage, Storage};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate};

#[component]
pub fn Deregister() -> impl IntoView {
    let notification = use_notify();
    let location = use_location();
    let navigate = use_navigate();
    let verified =
        use_context::<RwSignal<Option<bool>>>().unwrap_or_else(|| RwSignal::new(Some(false)));
    let url_params = location.query.get_untracked();
    let email = RwSignal::new(url_params.get("email").unwrap_or_default());
    let token_input = RwSignal::new(url_params.get("token").unwrap_or_default());
    let pathname = location.pathname.get_untracked();

    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            for (key, value) in [("email", email.get()), ("token", token_input.get())] {
                if !value.is_empty() {
                    pairs.push(format!("{}={}", key, crate::req::url_encode(&value)));
                }
            }
            let query_string = pairs.join("&");
            navigate(
                &format!("{pathname}?{query_string}"),
                leptos_router::NavigateOptions {
                    replace: true,
                    resolve: false,
                    ..Default::default()
                },
            );
        }
    };

    Effect::new(move |prev: Option<()>| {
        let _ = (email.get(), token_input.get());
        if prev.is_none() {
            return;
        }
        sync_url();
    });

    let sending = RwSignal::new(false);
    let confirming = RwSignal::new(false);

    let on_send = move |ev: SubmitEvent| {
        ev.prevent_default();
        if sending.get() || confirming.get() {
            return;
        }
        let email_val = email.get();
        let session_token = LocalStorage::get::<String>(SESSION_TOKEN_KEY).unwrap_or_default();
        if email_val.is_empty() {
            notify_error(&notification, "enter your email");
            return;
        }
        if session_token.is_empty() {
            notify_error(&notification, "authenticate first");
            return;
        }
        sending.set(true);
        spawn_local(async move {
            let challenge = match get_challenge().await {
                Ok(c) => c,
                Err(_) => {
                    notify_error(&notification, "connection issue");
                    sending.set(false);
                    return;
                }
            };
            let pow = match prove(ProveInput {
                challenge,
                payload: email_val.clone(),
            })
            .await
            {
                Ok(p) => p,
                Err(_) => {
                    notify_error(&notification, "verification failed");
                    sending.set(false);
                    return;
                }
            };
            match post_deregister_user(&pow, &session_token).await {
                Ok(_) => {
                    notify_success(&notification, "verification email sent");
                }
                Err(_) => {
                    notify_error(&notification, "deregister failed");
                }
            }
            sending.set(false);
        });
    };

    let on_confirm = move |ev: SubmitEvent| {
        ev.prevent_default();
        if confirming.get() {
            return;
        }
        let token_val = token_input.get().trim().to_string();
        let session_token = LocalStorage::get::<String>(SESSION_TOKEN_KEY).unwrap_or_default();
        if token_val.is_empty() {
            notify_error(&notification, "paste the token from your email");
            return;
        }
        if session_token.is_empty() {
            notify_error(&notification, "authenticate first");
            return;
        }
        if confirming.get() || sending.get() {
            return;
        }
        confirming.set(true);
        spawn_local(async move {
            let challenge = match get_challenge().await {
                Ok(c) => c,
                Err(_) => {
                    notify_error(&notification, "connection issue");
                    confirming.set(false);
                    return;
                }
            };
            let pow = match prove(ProveInput {
                challenge,
                payload: token_val.clone(),
            })
            .await
            {
                Ok(p) => p,
                Err(_) => {
                    notify_error(&notification, "verification failed");
                    confirming.set(false);
                    return;
                }
            };
            match post_deregister_user_confirm(&pow, &session_token).await {
                Ok(_) => {
                    LocalStorage::delete(SESSION_TOKEN_KEY);
                    notify_success(&notification, "account deregistered");
                    verified.set(Some(false));
                }
                Err(_) => {
                    notify_error(&notification, "deregister failed");
                }
            }
            confirming.set(false);
        });
    };

    view! {
        <form on:submit=on_send>
            <input placeholder="email" required maxlength="254" type="email" disabled=move || sending.get() bind:value=email/>
            <button type="submit" disabled=move || sending.get()>
                {move || if sending.get() { "sending..." } else { "send" }}
            </button>
        </form>
        <form on:submit=on_confirm>
            <input placeholder="token" required disabled=move || confirming.get() bind:value=token_input/>
            <button type="submit" disabled=move || confirming.get()>
                {move || if confirming.get() { "deregistering..." } else { "deregister" }}
            </button>
        </form>
    }
}
