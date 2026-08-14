use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::{authenticated_user_id, refresh_session};

#[component]
pub fn EmailUpdate() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let query = use_query_map();
    let old_token = RwSignal::new(query.get_untracked().get("old_token").unwrap_or_default());
    let new_token = RwSignal::new(query.get_untracked().get("new_token").unwrap_or_default());
    let working = RwSignal::new(false);

    persist_draft(
        navigate.clone(),
        "/private/email/update".to_string(),
        move || {
            vec![
                ("old_token", old_token.get()),
                ("new_token", new_token.get()),
            ]
        },
    );

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
        <form on:submit=submit>
            <input type="text" prop:value=old_token on:input=move |event| old_token.set(event_target_value(&event)) placeholder="token(old)"/>
            <input type="text" prop:value=new_token on:input=move |event| new_token.set(event_target_value(&event)) placeholder="token(new)"/>
            <button type="submit" disabled=move || working.get()>update</button>
        </form>
    }
}
