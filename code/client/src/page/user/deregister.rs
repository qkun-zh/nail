use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use common::request::DeleteMode;

use crate::page::confirm::use_confirm_action;
use crate::page::draft::mirror_text_param;
use crate::page::fetch::LoadError;
use crate::page::notify::{notify_success, use_notifications};
use crate::page::session_gate::{authenticated_user_id, mark_session_invalid};

#[component]
pub fn Deregister() -> impl IntoView {
    let notifications = use_notifications();
    let query = use_query_map();
    let email = RwSignal::new(query.get_untracked().get("email").unwrap_or_default());
    let token = RwSignal::new(query.get_untracked().get("token").unwrap_or_default());
    let mode = RwSignal::new(DeleteMode::Transfer);

    mirror_text_param("email", move || email.get());
    mirror_text_param("token", move || token.get());

    let send_notifications = notifications.clone();
    let send = use_confirm_action(move || {
        let value = email.get_untracked();
        let notifications = send_notifications.clone();
        async move {
            if value.trim().is_empty() {
                return Err(LoadError::from("enter your account email"));
            }
            crate::request::user::send_deregister_email(value).await?;
            notify_success(&notifications, "confirmation email sent");
            Ok(())
        }
    });

    let confirm_notifications = notifications.clone();
    let confirm = use_confirm_action(move || {
        let user_id = authenticated_user_id();
        let token_value = token.get_untracked().trim().to_string();
        let delete_mode = mode.get_untracked();
        let notifications = confirm_notifications.clone();
        async move {
            let Some(user_id) = user_id else {
                return Err(LoadError::from("authenticate to deregister"));
            };
            if token_value.is_empty() {
                return Err(LoadError::from("paste the confirmation token"));
            }
            crate::request::user::deregister_self(&user_id, token_value, delete_mode).await?;
            crate::request::session::clear_session_token();
            mark_session_invalid();
            notify_success(&notifications, "account deregistered");
            Ok(())
        }
    });

    view! {
        <form on:submit=move |event| {
            event.prevent_default();
            send.submit.run(());
        }>
            <input type="text" prop:value=email on:input=move |event| email.set(event_target_value(&event)) placeholder="email"/>
            <button type="submit" disabled=send.working>
                {move || if send.working.get() { "sending..." } else { "send" }}
            </button>
        </form>
        {move || send.error.get().map(|error| view! { <p class="error">{error}</p> })}
        <form on:submit=move |event| {
            event.prevent_default();
            confirm.submit.run(());
        }>
            <input type="text" prop:value=token on:input=move |event| token.set(event_target_value(&event)) placeholder="token"/>
            <label><input type="radio" name="mode" value="transfer" prop:checked=true on:change=move |_| mode.set(DeleteMode::Transfer)/> Transfer (content moves to platform)</label>
            <label><input type="radio" name="mode" value="soft" on:change=move |_| mode.set(DeleteMode::Soft)/> Soft (data preserved, admin can restore)</label>
            <button type="submit" disabled=confirm.working>
                {move || if confirm.working.get() { "deregistering..." } else { "deregister" }}
            </button>
        </form>
        {move || confirm.error.get().map(|error| view! { <p class="error">{error}</p> })}
    }
}
