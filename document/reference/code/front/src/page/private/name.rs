use crate::page::auth_gate::who_are_you;
use crate::page::notify::{notify_error, use_notify};
use crate::req::{SESSION_TOKEN_KEY, get_session};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

pub mod update;

#[component]
pub fn Name() -> impl IntoView {
    let notification = use_notify();
    let verified =
        use_context::<RwSignal<Option<bool>>>().unwrap_or_else(|| RwSignal::new(Some(false)));

    let current_name = RwSignal::new(None::<String>);
    let name_loaded = RwSignal::new(false);

    Effect::new(move |_| {
        if verified.get() == Some(true) && !name_loaded.get() {
            name_loaded.set(true);
            let token = match LocalStorage::get::<String>(SESSION_TOKEN_KEY) {
                Ok(t) => t,
                Err(e) => {
                    notify_error(&notification, &format!("failed to read session token: {e}"));
                    return;
                }
            };
            if token.is_empty() {
                notify_error(&notification, "not logged in");
                return;
            }
            spawn_local(async move {
                match get_session(&token, false, true).await {
                    Ok(data) => {
                        let name = data
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        current_name.set(name);
                    }
                    Err(e) => {
                        notify_error(&notification, &format!("load name failed: {e}"));
                    }
                }
            });
        }
    });

    view! {
        {move || match verified.get() {
            Some(true) => {
                let name = current_name.get();
                let greeting = match &name {
                    Some(n) if !n.is_empty() => format!("hi, {n}!"),
                    _ => "hi!".to_string(),
                };
                view! {
                    <p>{greeting}</p>
                    <A href="/private/name/update">update</A>
                }.into_any()
            }
            None => view! { <p>loading...</p> }.into_any(),
            _ => who_are_you(),
        }}
    }
}
