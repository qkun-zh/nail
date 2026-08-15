use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::{authenticated_user_id, refresh_session};

#[component]
pub fn EmailUpdate() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let query = use_query_map();
    let old_email = RwSignal::new(query.get_untracked().get("old_email").unwrap_or_default());
    let new_email = RwSignal::new(query.get_untracked().get("new_email").unwrap_or_default());
    let old_token = RwSignal::new(query.get_untracked().get("old_token").unwrap_or_default());
    let new_token = RwSignal::new(query.get_untracked().get("new_token").unwrap_or_default());
    let sending = RwSignal::new(false);
    let confirming = RwSignal::new(false);

    persist_draft(
        navigate.clone(),
        "/private/email/update".to_string(),
        move || {
            vec![
                ("old_email", old_email.get()),
                ("new_email", new_email.get()),
                ("old_token", old_token.get()),
                ("new_token", new_token.get()),
            ]
        },
    );

    let send_notifications = notifications.clone();
    let send = move |event: SubmitEvent| {
        event.prevent_default();
        if sending.get() || confirming.get() {
            return;
        }
        let old_email_value = old_email.get();
        let new_email_value = new_email.get();
        if old_email_value.trim().is_empty() || new_email_value.trim().is_empty() {
            notify_error(&send_notifications, "enter both the old and the new email");
            return;
        }
        if old_email_value == new_email_value {
            notify_error(
                &send_notifications,
                "the new email must differ from the old one",
            );
            return;
        }
        sending.set(true);
        let notifications = send_notifications.clone();
        leptos::task::spawn_local(async move {
            let result = async {
                let old_pow = crate::request::pow::prove_pow(old_email_value.clone()).await?;
                let new_pow = crate::request::pow::prove_pow(new_email_value.clone()).await?;
                crate::request::user::send_change_email(old_pow, new_pow).await
            }
            .await;
            match result {
                Ok(_) => notify_success(&notifications, "confirmation emails sent"),
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            sending.set(false);
        });
    };

    let confirm_notifications = notifications.clone();
    let confirm = move |event: SubmitEvent| {
        event.prevent_default();
        if confirming.get() || sending.get() {
            return;
        }
        let Some(user_id) = authenticated_user_id() else {
            notify_error(&confirm_notifications, "authenticate to change email");
            return;
        };
        let old_token_value = old_token.get().trim().to_string();
        let new_token_value = new_token.get().trim().to_string();
        if old_token_value.is_empty() || new_token_value.is_empty() {
            notify_error(&confirm_notifications, "paste both emailed tokens");
            return;
        }
        if old_token_value == new_token_value {
            notify_error(&confirm_notifications, "the two tokens must differ");
            return;
        }
        let payload = format!("{old_token_value}\n{new_token_value}");
        confirming.set(true);
        let notifications = confirm_notifications.clone();
        leptos::task::spawn_local(async move {
            let result = match crate::request::pow::prove_pow(payload).await {
                Ok(pow) => {
                    crate::request::user::confirm_email_change(
                        &user_id,
                        pow,
                        &old_token_value,
                        &new_token_value,
                    )
                    .await
                }
                Err(error) => Err(error),
            };
            match result {
                Ok(view) => {
                    crate::request::session::store_session_token(&view.session_token);
                    refresh_session();
                    notify_success(&notifications, "email changed");
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            confirming.set(false);
        });
    };

    view! {
        <form on:submit=send>
            <input type="text" prop:value=old_email on:input=move |event| old_email.set(event_target_value(&event)) placeholder="email(old)"/>
            <input type="text" prop:value=new_email on:input=move |event| new_email.set(event_target_value(&event)) placeholder="email(new)"/>
            <button type="submit" disabled=move || sending.get()>
                {move || if sending.get() { "sending..." } else { "send" }}
            </button>
        </form>
        <form on:submit=confirm>
            <input type="text" prop:value=old_token on:input=move |event| old_token.set(event_target_value(&event)) placeholder="token(old)"/>
            <input type="text" prop:value=new_token on:input=move |event| new_token.set(event_target_value(&event)) placeholder="token(new)"/>
            <button type="submit" disabled=move || confirming.get()>
                {move || if confirming.get() { "updating..." } else { "update" }}
            </button>
        </form>
    }
}
