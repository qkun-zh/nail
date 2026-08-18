use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::page::notify::{notify_error, use_notifications};

#[component]
pub fn UserRole() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let uid = move || params.get().get("uid").unwrap_or_default();
    let roles = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let id = uid();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::user::read_user(&id).await {
                Ok(view) => {
                    let text = view.roles.unwrap_or_default().join(", ");
                    roles.set(Some(text));
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    });

    view! {
        <p>{move || roles.get().unwrap_or_default()}</p>
    }
}
