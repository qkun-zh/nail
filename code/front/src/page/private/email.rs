pub mod update;

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};

#[component]
pub fn EmailIndex() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let query = use_query_map();
    let old_email = RwSignal::new(query.get_untracked().get("old_email").unwrap_or_default());
    let new_email = RwSignal::new(query.get_untracked().get("new_email").unwrap_or_default());
    let working = RwSignal::new(false);

    persist_draft(navigate.clone(), "/private/email".to_string(), move || {
        vec![
            ("old_email", old_email.get()),
            ("new_email", new_email.get()),
        ]
    });

    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let old_email_value = old_email.get();
        let new_email_value = new_email.get();
        if old_email_value.trim().is_empty() || new_email_value.trim().is_empty() {
            notify_error(&notifications, "enter both the old and the new email");
            return;
        }
        if old_email_value == new_email_value {
            notify_error(&notifications, "the new email must differ from the old one");
            return;
        }
        working.set(true);
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            let result = async {
                let old_pow = crate::request::pow::prove_pow(old_email_value.clone()).await?;
                let new_pow = crate::request::pow::prove_pow(new_email_value.clone()).await?;
                crate::request::user::send_change_email(old_pow, new_pow).await
            }
            .await;
            match result {
                Ok(_) => {
                    notify_success(&notifications, "confirmation emails sent");
                    navigate(
                        "/private/email/update",
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
            <input type="text" prop:value=old_email on:input=move |event| old_email.set(event_target_value(&event)) placeholder="email(old)"/>
            <input type="text" prop:value=new_email on:input=move |event| new_email.set(event_target_value(&event)) placeholder="email(new)"/>
            <button type="submit" disabled=move || working.get()>
                {move || if working.get() { "sending..." } else { "send" }}
            </button>
        </form>
    }
}
