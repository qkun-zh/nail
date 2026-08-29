use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::page::confirm::use_confirm_action;
use crate::page::draft::mirror_text_param;
use crate::page::fetch::LoadError;
use crate::page::notify::{notify_success, use_notifications};
use crate::page::panel::{
    PanelField, PanelForm, PanelFrame, PanelInner, PanelInput, PanelPage, PanelSubmit, PanelTitle,
};
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
        <PanelPage>
            <PanelFrame>
                <PanelInner>
                    <PanelTitle>"Authenticate"</PanelTitle>
                    <PanelForm>
                        <form class="contents" on:submit=move |event| {
                            event.prevent_default();
                            send.submit.run(());
                        }>
                            <PanelField>
                                <PanelInput value=email on_input=move |v| email.set(v) placeholder="email" autocomplete="email" />
                            </PanelField>
                            <PanelSubmit disabled=send.working>
                                {move || if send.working.get() { "sending..." } else { "send" }}
                            </PanelSubmit>
                        </form>
                    </PanelForm>
                    <PanelForm next=true>
                        <form class="contents" on:submit=move |event| {
                            event.prevent_default();
                            redeem.submit.run(());
                        }>
                            <PanelField>
                                <PanelInput value=token on_input=move |v| token.set(v) placeholder="token" />
                            </PanelField>
                            <PanelSubmit disabled=redeem.working>
                                {move || if redeem.working.get() { "authenticating..." } else { "authenticate" }}
                            </PanelSubmit>
                        </form>
                    </PanelForm>
                </PanelInner>
            </PanelFrame>
        </PanelPage>
    }
}
