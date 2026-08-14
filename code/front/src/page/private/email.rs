pub mod update;

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;

use crate::page::notify::{notify_error, notify_success, use_notifications};

#[component]
pub fn EmailIndex() -> impl IntoView {
    let navigate = use_navigate();
    let notifications = use_notifications();
    let old_email = RwSignal::new(String::new());
    let new_email = RwSignal::new(String::new());
    let working = RwSignal::new(false);

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
            <div>
                <p>change email</p>
                <form on:submit=submit>
                    <input type="text" prop:value=old_email on:input=move |event| old_email.set(event_target_value(&event)) placeholder="old_email"/>
                    <input type="text" prop:value=new_email on:input=move |event| new_email.set(event_target_value(&event)) placeholder="new_email"/>
                    <button type="submit" disabled=move || working.get()>send</button>
                </form>
            </div>
    }
}
