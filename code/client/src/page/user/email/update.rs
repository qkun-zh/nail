use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

use crate::page::confirm::use_confirm_action;
use crate::page::draft::mirror_text_param;
use crate::page::fetch::LoadError;
use crate::page::notify::{notify_success, use_notifications};
use crate::page::session_gate::{authenticated_user_id, refresh_session};

#[component]
pub fn EmailUpdate() -> impl IntoView {
    let notifications = use_notifications();
    let query = use_query_map();
    let params = use_params_map();
    let old_email = RwSignal::new(query.get_untracked().get("old_email").unwrap_or_default());
    let new_email = RwSignal::new(query.get_untracked().get("new_email").unwrap_or_default());
    let old_token = RwSignal::new(query.get_untracked().get("old_token").unwrap_or_default());
    let new_token = RwSignal::new(query.get_untracked().get("new_token").unwrap_or_default());

    mirror_text_param("old_email", move || old_email.get());
    mirror_text_param("new_email", move || new_email.get());
    mirror_text_param("old_token", move || old_token.get());
    mirror_text_param("new_token", move || new_token.get());

    let send_notifications = notifications.clone();
    let send = use_confirm_action(move || {
        let old_email_value = old_email.get_untracked();
        let new_email_value = new_email.get_untracked();
        let notifications = send_notifications.clone();
        async move {
            if old_email_value.trim().is_empty() || new_email_value.trim().is_empty() {
                return Err(LoadError::from("enter both the old and the new email"));
            }
            if old_email_value == new_email_value {
                return Err(LoadError::from(
                    "the new email must differ from the old one",
                ));
            }
            crate::request::user::send_change_email(old_email_value, new_email_value).await?;
            notify_success(&notifications, "confirmation emails sent");
            Ok(())
        }
    });

    let confirm_notifications = notifications.clone();
    let confirm = use_confirm_action(move || {
        let user_id = authenticated_user_id();
        let route_uid = params.get_untracked().get("uid");
        let old_token_value = old_token.get_untracked().trim().to_string();
        let new_token_value = new_token.get_untracked().trim().to_string();
        let notifications = confirm_notifications.clone();
        async move {
            let Some(user_id) = user_id else {
                return Err(LoadError::from("authenticate to change email"));
            };
            if route_uid.is_some_and(|uid| uid != user_id) {
                return Err(LoadError::from("cannot change another user's email"));
            }
            if old_token_value.is_empty() || new_token_value.is_empty() {
                return Err(LoadError::from("paste both emailed tokens"));
            }
            if old_token_value == new_token_value {
                return Err(LoadError::from("the two tokens must differ"));
            }
            let view = crate::request::user::confirm_email_change(
                &user_id,
                &old_token_value,
                &new_token_value,
            )
            .await?;
            crate::request::session::store_session_token(&view.session_token);
            refresh_session();
            notify_success(&notifications, "email changed");
            Ok(())
        }
    });

    view! {
        <form on:submit=move |event| {
            event.prevent_default();
            send.submit.run(());
        }>
            <input type="text" prop:value=old_email on:input=move |event| old_email.set(event_target_value(&event)) placeholder="email(old)"/>
            <input type="text" prop:value=new_email on:input=move |event| new_email.set(event_target_value(&event)) placeholder="email(new)"/>
            <button type="submit" disabled=send.working>
                {move || if send.working.get() { "sending..." } else { "send" }}
            </button>
        </form>
        {move || send.error.get().map(|error| view! { <p class="error">{error}</p> })}
        <form on:submit=move |event| {
            event.prevent_default();
            confirm.submit.run(());
        }>
            <input type="text" prop:value=old_token on:input=move |event| old_token.set(event_target_value(&event)) placeholder="token(old)"/>
            <input type="text" prop:value=new_token on:input=move |event| new_token.set(event_target_value(&event)) placeholder="token(new)"/>
            <button type="submit" disabled=confirm.working>
                {move || if confirm.working.get() { "updating..." } else { "update" }}
            </button>
        </form>
        {move || confirm.error.get().map(|error| view! { <p class="error">{error}</p> })}
    }
}
