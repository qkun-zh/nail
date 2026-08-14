use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::{authenticated_user_id, mark_session_invalid};

#[component]
pub fn Deregister() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let query = use_query_map();
    let email = RwSignal::new(query.get_untracked().get("email").unwrap_or_default());
    let token = RwSignal::new(query.get_untracked().get("token").unwrap_or_default());
    let working = RwSignal::new(false);

    persist_draft(
        navigate.clone(),
        "/private/deregister".to_string(),
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
            let result = match crate::request::pow::prove_pow(email_value).await {
                Ok(pow) => crate::request::user::send_deregister_email(pow).await,
                Err(error) => Err(error),
            };
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
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            let result = match crate::request::pow::prove_pow(token_value).await {
                Ok(pow) => crate::request::user::deregister_self(&user_id, pow).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(_) => {
                    crate::request::session::clear_session_token();
                    mark_session_invalid();
                    notify_success(&notifications, "account deregistered");
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
        <form on:submit=send_confirmation>
            <input type="text" prop:value=email on:input=move |event| email.set(event_target_value(&event)) placeholder="email"/>
            <button type="submit" disabled=move || working.get()>send</button>
        </form>
        <form on:submit=confirm>
            <input type="text" prop:value=token on:input=move |event| token.set(event_target_value(&event)) placeholder="token"/>
            <button type="submit" disabled=move || working.get()>deregister</button>
        </form>
    }
}
