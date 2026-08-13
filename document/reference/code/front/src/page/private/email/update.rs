use crate::page::notify::{notify_error, notify_success, use_notify};
use crate::pow::prove;
use crate::req::{
    ProveInput, SESSION_TOKEN_KEY, get_challenge, post_email_update_confirm, post_email_update_send,
};
use gloo_storage::{LocalStorage, Storage};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate};

#[component]
pub fn Update() -> impl IntoView {
    let notification = use_notify();
    let location = use_location();
    let navigate = use_navigate();
    let url_params = location.query.get_untracked();
    let old_email = RwSignal::new(url_params.get("old_email").unwrap_or_default());
    let new_email = RwSignal::new(url_params.get("new_email").unwrap_or_default());
    let old_token = RwSignal::new(url_params.get("old_token").unwrap_or_default());
    let new_token = RwSignal::new(url_params.get("new_token").unwrap_or_default());
    let pathname = location.pathname.get_untracked();

    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            for (key, value) in [
                ("old_email", old_email.get()),
                ("new_email", new_email.get()),
                ("old_token", old_token.get()),
                ("new_token", new_token.get()),
            ] {
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
        let _ = (
            old_email.get(),
            new_email.get(),
            old_token.get(),
            new_token.get(),
        );
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
        let old_email_val = old_email.get();
        let new_email_val = new_email.get();
        let session_token = LocalStorage::get::<String>(SESSION_TOKEN_KEY).unwrap_or_default();
        if old_email_val.is_empty() || new_email_val.is_empty() {
            notify_error(&notification, "enter old email and new email");
            return;
        }
        if old_email_val == new_email_val {
            notify_error(&notification, "old email and new email must differ");
            return;
        }
        if session_token.is_empty() {
            notify_error(&notification, "authenticate first");
            return;
        }
        sending.set(true);
        spawn_local(async move {
            let challenge_old = match get_challenge().await {
                Ok(c) => c,
                Err(_) => {
                    notify_error(&notification, "connection issue");
                    sending.set(false);
                    return;
                }
            };
            let challenge_new = match get_challenge().await {
                Ok(c) => c,
                Err(_) => {
                    notify_error(&notification, "connection issue");
                    sending.set(false);
                    return;
                }
            };
            let pow_old = match prove(ProveInput {
                challenge: challenge_old,
                payload: old_email_val.clone(),
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
            let pow_new = match prove(ProveInput {
                challenge: challenge_new,
                payload: new_email_val.clone(),
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
            match post_email_update_send(&pow_old, &pow_new, &session_token).await {
                Ok(_) => {
                    notify_success(&notification, "verification emails sent");
                }
                Err(e) => {
                    notify_error(&notification, &format!("send failed: {e}"));
                }
            }
            sending.set(false);
        });
    };

    let on_confirm = move |ev: SubmitEvent| {
        ev.prevent_default();
        if confirming.get() || sending.get() {
            return;
        }
        let old_token_val = old_token.get().trim().to_string();
        let new_token_val = new_token.get().trim().to_string();
        let session_token = LocalStorage::get::<String>(SESSION_TOKEN_KEY).unwrap_or_default();
        if old_token_val.is_empty() || new_token_val.is_empty() {
            notify_error(&notification, "paste old token and new token");
            return;
        }
        if old_token_val == new_token_val {
            notify_error(&notification, "old token and new token must differ");
            return;
        }
        if session_token.is_empty() {
            notify_error(&notification, "authenticate first");
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
                payload: format!("{}\n{}", old_token_val, new_token_val),
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
            match post_email_update_confirm(&pow, &old_token_val, &new_token_val, &session_token)
                .await
            {
                Ok(data) => {
                    match data.get("session_token").and_then(|v| v.as_str()) {
                        Some(new_session_token) => {
                            match LocalStorage::set(SESSION_TOKEN_KEY, new_session_token) {
                                Ok(()) => notify_success(&notification, "email updated"),
                                Err(e) => notify_error(
                                    &notification,
                                    &format!(
                                        "email updated but failed to persist new session token: {e}"
                                    ),
                                ),
                            }
                        }
                        None => {
                            notify_error(
                                &notification,
                                "email updated but no new session token issued, please authenticate again",
                            );
                        }
                    }
                }
                Err(e) => {
                    notify_error(&notification, &format!("email update failed: {e}"));
                }
            }
            confirming.set(false);
        });
    };

    view! {
        <form on:submit=on_send>
            <input placeholder="email(old)" required maxlength="254" type="email" disabled=move || sending.get() bind:value=old_email/>
            <input placeholder="email(new)" required maxlength="254" type="email" disabled=move || sending.get() bind:value=new_email/>
            <button type="submit" disabled=move || sending.get()>
                {move || if sending.get() { "sending..." } else { "send" }}
            </button>
        </form>
        <form on:submit=on_confirm>
            <input placeholder="token(old)" required disabled=move || confirming.get() bind:value=old_token/>
            <input placeholder="token(new)" required disabled=move || confirming.get() bind:value=new_token/>
            <button type="submit" disabled=move || confirming.get()>
                {move || if confirming.get() { "updating..." } else { "update" }}
            </button>
        </form>
    }
}
