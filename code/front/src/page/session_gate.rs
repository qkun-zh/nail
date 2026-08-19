use std::sync::OnceLock;

use leptos::prelude::*;
use leptos_router::components::A;
use nail_common::response::session::SessionView;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    Checking,
    Authenticated(SessionView),
    Anonymous,
}

static SESSION_STATUS: OnceLock<RwSignal<SessionStatus>> = OnceLock::new();

pub fn provide_session_state() -> RwSignal<SessionStatus> {
    let status = RwSignal::new(SessionStatus::Checking);
    let _ = SESSION_STATUS.set(status);
    provide_context(status);
    verify_once(status);
    status
}

pub fn use_session_status() -> RwSignal<SessionStatus> {
    use_context::<RwSignal<SessionStatus>>()
        .or_else(|| SESSION_STATUS.get().copied())
        .unwrap_or_else(|| RwSignal::new(SessionStatus::Anonymous))
}

pub fn mark_session_invalid() {
    if let Some(status) = SESSION_STATUS.get() {
        status.set(SessionStatus::Anonymous);
    }
}

pub fn authenticated_user_id() -> Option<String> {
    let status = SESSION_STATUS.get()?;
    match status.get_untracked() {
        SessionStatus::Authenticated(view) => view.id.clone(),
        _ => None,
    }
}

pub fn refresh_session() {
    let Some(status) = SESSION_STATUS.get() else {
        return;
    };
    leptos::task::spawn_local(async move {
        match crate::request::auth::read_session(true, true).await {
            Ok(view) => status.set(SessionStatus::Authenticated(view)),
            Err(_) => status.set(SessionStatus::Anonymous),
        }
    });
}

fn verify_once(status: RwSignal<SessionStatus>) {
    leptos::task::spawn_local(async move {
        let result = match crate::request::session::read_session_token() {
            Some(token) if !token.is_empty() => {
                match crate::request::auth::read_session(true, true).await {
                    Ok(view) => SessionStatus::Authenticated(view),
                    Err(_) => SessionStatus::Anonymous,
                }
            }
            _ => SessionStatus::Anonymous,
        };
        status.set(result);
    });
}

pub fn who_are_you() -> AnyView {
    view! {
        <p>who are you?</p>
        <div><A href="/authenticate">authenticate</A></div>
    }
    .into_any()
}
