use crate::page::notify::{notify_error, notify_success, use_notify};
use crate::pow::prove;
use crate::req::{ProveInput, SESSION_TOKEN_KEY, get_challenge, post_email_read, post_user_create};
use gloo_storage::{LocalStorage, Storage};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate};

#[component]
pub fn Authenticate() -> impl IntoView {
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
        if sending.get() {
            return;
        }
        let email_val = email.get();
        if email_val.is_empty() {
            notify_error(&notification, "enter your email");
            return;
        }
        sending.set(true);
        spawn_local(async move {
            let challenge = match get_challenge().await {
                Ok(c) => c,
                Err(e) => {
                    notify_error(&notification, &format!("get challenge failed: {e}"));
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
                Err(e) => {
                    notify_error(&notification, &format!("proof failed: {e}"));
                    sending.set(false);
                    return;
                }
            };
            match post_email_read(&pow).await {
                Ok(_) => {
                    notify_success(&notification, "verification email sent");
                }
                Err(e) => {
                    notify_error(&notification, &e.to_string());
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
        if token_val.is_empty() {
            notify_error(&notification, "paste the token from your email");
            return;
        }
        confirming.set(true);
        spawn_local(async move {
            let challenge = match get_challenge().await {
                Ok(c) => c,
                Err(e) => {
                    notify_error(&notification, &format!("get challenge failed: {e}"));
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
                Err(e) => {
                    notify_error(&notification, &format!("proof failed: {e}"));
                    confirming.set(false);
                    return;
                }
            };
            match post_user_create(&pow).await {
                Ok(data) => {
                    match data.get("session_token").and_then(|v| v.as_str()) {
                        Some(session_token) => {
                            match LocalStorage::set(SESSION_TOKEN_KEY, session_token) {
                                Ok(()) => {
                                    notify_success(&notification, "authenticated");
                                    verified.set(Some(true));
                                }
                                Err(e) => {
                                    notify_error(
                                        &notification,
                                        &format!("failed to save session token: {e}"),
                                    );
                                }
                            }
                        }
                        None => {
                            notify_error(&notification, "session not issued, try again");
                        }
                    }
                }
                Err(e) => {
                    notify_error(&notification, &e.to_string());
                }
            }
            confirming.set(false);
        });
    };

    view! {
        <form on:submit=on_send>
            <input required maxlength="254" type="email" placeholder="email" disabled=move || sending.get() bind:value=email/>
            <button type="submit" disabled=move || sending.get()>
                {move || if sending.get() { "sending..." } else { "send" }}
            </button>
        </form>
        <form on:submit=on_confirm>
            <input placeholder="token" required disabled=move || confirming.get() bind:value=token_input/>
            <button type="submit" disabled=move || confirming.get()>
                {move || if confirming.get() { "authenticating..." } else { "authenticate" }}
            </button>
        </form>
    }
}
