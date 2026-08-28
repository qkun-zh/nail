use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::page::confirm::use_confirm_action;
use crate::page::draft::mirror_text_param;
use crate::page::fetch::LoadError;
use crate::page::notify::{notify_success, use_notifications};
use crate::page::session_gate::refresh_session;

#[component]
pub fn Authenticate() -> impl IntoView {
    let notifications = use_notifications();
    let query = use_query_map();

    let email = RwSignal::new(query.get_untracked().get("email").unwrap_or_default());
    let token = RwSignal::new(query.get_untracked().get("token").unwrap_or_default());

    mirror_text_param("email", move || email.get());
    mirror_text_param("token", move || token.get());

    let send_notifications = notifications.clone();
    let send = use_confirm_action(move || {
        let value = email.get_untracked();
        let notifications = send_notifications.clone();
        async move {
            if value.trim().is_empty() {
                return Err(LoadError::from("enter your email"));
            }
            let view = crate::request::auth::send_authenticate_email(value).await?;
            notify_success(
                &notifications,
                format!("email sent: {}", view.email_subject),
            );
            Ok(())
        }
    });

    let redeem_notifications = notifications.clone();
    let redeem = use_confirm_action(move || {
        let value = token.get_untracked().trim().to_string();
        let notifications = redeem_notifications.clone();
        async move {
            if value.is_empty() {
                return Err(LoadError::from("paste the emailed token"));
            }
            let view = crate::request::auth::redeem_token(value).await?;
            crate::request::session::store_session_token(&view.session_token);
            refresh_session();
            notify_success(&notifications, "signed in");
            Ok(())
        }
    });

    view! {
        <div class="panel-page">
            <div class="panel-frame">
                <div class="panel-inner">
                    <h1 class="panel-title">"Authenticate"</h1>
                    <form class="panel-form" on:submit=move |event| {
                        event.prevent_default();
                        send.submit.run(());
                    }>
                        <div class="panel-field">
                            <input class="panel-input" type="text" prop:value=email on:input=move |event| email.set(event_target_value(&event)) placeholder="email" autocomplete="email" spellcheck="false" />
                        </div>
                        <button class="panel-submit" type="submit" disabled=send.working>
                            {move || if send.working.get() { "sending..." } else { "send" }}
                        </button>
                    </form>
                    <form class="panel-form" on:submit=move |event| {
                        event.prevent_default();
                        redeem.submit.run(());
                    }>
                        <div class="panel-field">
                            <input class="panel-input" type="text" prop:value=token on:input=move |event| token.set(event_target_value(&event)) placeholder="token" spellcheck="false" />
                        </div>
                        <button class="panel-submit" type="submit" disabled=redeem.working>
                            {move || if redeem.working.get() { "authenticating..." } else { "authenticate" }}
                        </button>
                    </form>
                </div>
            </div>
        </div>
    }
}
