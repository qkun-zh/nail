use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::page::draft::mirror_text_param;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::refresh_session;

#[component]
pub fn Authenticate() -> impl IntoView {
    let notifications = use_notifications();
    let query = use_query_map();

    let email = RwSignal::new(query.get_untracked().get("email").unwrap_or_default());
    let token = RwSignal::new(query.get_untracked().get("token").unwrap_or_default());
    let working = RwSignal::new(false);

    mirror_text_param("email", move || email.get());
    mirror_text_param("token", move || token.get());

    let send_notifications = notifications.clone();
    let send_email = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let email_value = email.get();
        if email_value.trim().is_empty() {
            notify_error(&send_notifications, "enter your email");
            return;
        }
        working.set(true);
        let notifications = send_notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::auth::send_authenticate_email(email_value).await;
            match result {
                Ok(view) => notify_success(
                    &notifications,
                    format!("email sent: {}", view.email_subject),
                ),
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            working.set(false);
        });
    };

    let redeem_notifications = notifications.clone();
    let redeem = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let token_value = token.get().trim().to_string();
        if token_value.is_empty() {
            notify_error(&redeem_notifications, "paste the emailed token");
            return;
        }
        working.set(true);
        let notifications = redeem_notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::auth::redeem_token(token_value).await;
            match result {
                Ok(view) => {
                    crate::request::session::store_session_token(&view.session_token);
                    refresh_session();
                    notify_success(&notifications, "signed in");
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            working.set(false);
        });
    };

    view! {
        <form on:submit=send_email>
            <input type="text" prop:value=email on:input=move |event| email.set(event_target_value(&event)) placeholder="email"/>
            <button type="submit" disabled=move || working.get()>
                {move || if working.get() { "sending..." } else { "send" }}
            </button>
        </form>
        <form on:submit=redeem>
            <input type="text" prop:value=token on:input=move |event| token.set(event_target_value(&event)) placeholder="token"/>
            <button type="submit" disabled=move || working.get()>
                {move || if working.get() { "authenticating..." } else { "authenticate" }}
            </button>
        </form>
    }
}
