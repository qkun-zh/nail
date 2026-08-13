
use crate::page::notify::{notify_error, use_notify};
use crate::req::{SESSION_TOKEN_KEY, check_is_author, get_session};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_location;
use std::sync::OnceLock;

pub const AUTHENTICATE_PATH: &str = "/private/authenticate";

pub fn who_are_you() -> AnyView {
    view! {
        <p>who are you?</p>
        <A href=AUTHENTICATE_PATH>authenticate</A>
    }
    .into_any()
}

pub fn denied_view() -> AnyView {
    view! { <p>you are denied!</p> }.into_any()
}

struct SessionVerified {
    #[allow(dead_code)]
    owner: Owner,
    verified: RwSignal<Option<bool>>,
}

static SESSION_VERIFIED: OnceLock<SessionVerified> = OnceLock::new();

pub fn get_session_verified() -> RwSignal<Option<bool>> {
    SESSION_VERIFIED
        .get_or_init(|| {
            let prev = Owner::current();
            let owner = Owner::new_root(None);
            let verified = owner.with(|| {
                let verified = RwSignal::new(None::<bool>);
                spawn_local(async move {
                    let session_token =
                        LocalStorage::get::<String>(SESSION_TOKEN_KEY).unwrap_or_default();
                    if session_token.is_empty() {
                        verified.set(Some(false));
                        return;
                    }
                    match get_session(&session_token, false, false).await {
                        Ok(_) => verified.set(Some(true)),
                        Err(_) => verified.set(Some(false)),
                    }
                });
                verified
            });
            if let Some(prev) = prev {
                prev.set();
            }
            SessionVerified { owner, verified }
        })
        .verified
}

pub fn mark_session_invalid() {
    get_session_verified().set(Some(false));
}

pub fn use_component_alive() -> ArcStoredValue<bool> {
    let alive = ArcStoredValue::new(true);
    on_cleanup({
        let alive = alive.clone();
        move || alive.set_value(false)
    });
    alive
}

#[component]
pub fn SessionGate(
    #[prop(optional)] always: Option<&'static str>,
    children: ChildrenFn,
) -> impl IntoView {
    let verified = get_session_verified();
    provide_context(verified);
    let location = use_location();
    view! {
        {move || {
            if let Some(path) = always
                && location.pathname.get() == path
            {
                return children().into_any();
            }
            match verified.get() {
                None => view! { <p>checking session...</p> }.into_any(),
                Some(true) => children().into_any(),
                Some(false) => who_are_you(),
            }
        }}
    }
}

pub fn use_author_gate<F>(targets: F) -> (RwSignal<bool>, RwSignal<bool>)
where
    F: Fn() -> Option<(Option<String>, Option<String>, Option<String>)> + 'static,
{
    let notification = use_notify();
    let alive = use_component_alive();
    let denied = RwSignal::new(false);
    let checked = RwSignal::new(false);
    let seq = StoredValue::new(0u64);
    Effect::new(move |_| {
        let my_seq = seq.get_value() + 1;
        seq.set_value(my_seq);

        let Some((article_id, version_id, comment_id)) = targets() else {
            denied.set(false);
            checked.set(true);
            return;
        };
        let has_target = [&article_id, &version_id, &comment_id]
            .into_iter()
            .any(|t| t.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false));
        if !has_target {
            denied.set(false);
            checked.set(true);
            return;
        }
        checked.set(false);
        let token = LocalStorage::get::<String>(SESSION_TOKEN_KEY).unwrap_or_default();
        let alive = alive.clone();
        spawn_local(async move {
            let result = check_is_author(
                &token,
                article_id.as_deref(),
                version_id.as_deref(),
                comment_id.as_deref(),
            )
            .await;
            if !alive.get_value() {
                return;
            }
            if seq.get_value() != my_seq {
                return;
            }
            match result {
                Ok(is_author) => denied.set(!is_author),
                Err(e) => notify_error(&notification, &format!("author check failed: {e}")),
            }
            checked.set(true);
        });
    });
    (denied, checked)
}
