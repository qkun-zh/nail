use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use nail_common::request::DeleteMode;

use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::{authenticated_user_id, mark_session_invalid};

#[component]
pub fn Deregister() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let query = use_query_map();
    let params = use_params_map();
    let email = RwSignal::new(query.get_untracked().get("email").unwrap_or_default());
    let token = RwSignal::new(query.get_untracked().get("token").unwrap_or_default());
    let mode = RwSignal::new(DeleteMode::Transfer);
    let working = RwSignal::new(false);

    persist_draft(
        navigate.clone(),
        format!(
            "/user/{}/deregister",
            params.get_untracked().get("uid").unwrap_or_default()
        ),
        move || vec![("email", email.get()), ("token", token.get())],
    );

    let send_notifications = notifications.clone();
    let send_confirmation = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let email_value = email.get();
        if email_value.trim().is_empty() {
            notify_error(&send_notifications, "enter your account email");
            return;
        }
        working.set(true);
        let notifications = send_notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::user::send_deregister_email(email_value).await;
            match result {
                Ok(_) => notify_success(&notifications, "confirmation email sent"),
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            working.set(false);
        });
    };

    let confirm_notifications = notifications.clone();
    let confirm = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let Some(user_id) = authenticated_user_id() else {
            notify_error(&confirm_notifications, "authenticate to deregister");
            return;
        };
        let token_value = token.get().trim().to_string();
        if token_value.is_empty() {
            notify_error(&confirm_notifications, "paste the confirmation token");
            return;
        }
        working.set(true);
        let notifications = confirm_notifications.clone();
        leptos::task::spawn_local(async move {
            let result =
                crate::request::user::deregister_self(&user_id, token_value, mode.get()).await;
            match result {
                Ok(_) => {
                    crate::request::session::clear_session_token();
                    mark_session_invalid();
                    notify_success(&notifications, "account deregistered");
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
            working.set(false);
        });
    };

    view! {
        <form on:submit=send_confirmation>
            <input type="text" prop:value=email on:input=move |event| email.set(event_target_value(&event)) placeholder="email"/>
            <button type="submit" disabled=move || working.get()>
                {move || if working.get() { "sending..." } else { "send" }}
            </button>
        </form>
        <form on:submit=confirm>
            <input type="text" prop:value=token on:input=move |event| token.set(event_target_value(&event)) placeholder="token"/>
            <label><input type="radio" name="mode" value="transfer" prop:checked=true on:change=move |_| mode.set(DeleteMode::Transfer)/> Transfer (content moves to platform)</label>
            <label><input type="radio" name="mode" value="soft" on:change=move |_| mode.set(DeleteMode::Soft)/> Soft (data preserved, admin can restore)</label>
            <button type="submit" disabled=move || working.get()>
                {move || if working.get() { "deregistering..." } else { "deregister" }}
            </button>
        </form>
    }
}
