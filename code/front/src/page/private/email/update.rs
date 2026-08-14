use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::{authenticated_user_id, refresh_session};

#[component]
pub fn EmailUpdate() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let old_token = RwSignal::new(String::new());
    let new_token = RwSignal::new(String::new());
    let working = RwSignal::new(false);

    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let Some(user_id) = authenticated_user_id() else {
            notify_error(&notifications, "authenticate to change email");
            return;
        };
        let old_token_value = old_token.get().trim().to_string();
        let new_token_value = new_token.get().trim().to_string();
        if old_token_value.is_empty() || new_token_value.is_empty() {
            notify_error(&notifications, "paste both emailed tokens");
            return;
        }
        if old_token_value == new_token_value {
            notify_error(&notifications, "the two tokens must differ");
            return;
        }
        let payload = format!("{old_token_value}\n{new_token_value}");
        working.set(true);
        let notifications = notifications.clone();
        let navigate = navigate.clone();
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
                    navigate(
                        "/private",
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
                <p>confirm email change</p>
                <form on:submit=submit>
                    <input type="text" prop:value=old_token on:input=move |event| old_token.set(event_target_value(&event)) placeholder="old email token"/>
                    <input type="text" prop:value=new_token on:input=move |event| new_token.set(event_target_value(&event)) placeholder="new email token"/>
                    <button type="submit" disabled=move || working.get()>confirm</button>
                </form>
            </div>
    }
}
