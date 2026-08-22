use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::validation::validate_uuid;

#[component]
pub fn UserRole() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let uid = move || params.get().get("uid").unwrap_or_default();
    let roles = RwSignal::new(None::<Vec<common::response::user::RoleRef>>);

    Effect::new(move |_| {
        let id = uid();
        let notifications = notifications.clone();
        if let Err(error) = validate_uuid(&id) {
            notify_error(&notifications, error);
            return;
        }
        leptos::task::spawn_local(async move {
            match crate::request::user::read_user(&id).await {
                Ok(view) => roles.set(Some(view.roles.unwrap_or_default())),
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    });

    view! {
        <ul>
            {move || match roles.get() {
                None => Vec::new(),
                Some(roles) => roles
                    .into_iter()
                    .map(|role| {
                        let name = role.name.clone();
                        view! {
                            <li>
                                <A href={format!("/role/{}", role.id)}>{name}</A>
                            </li>
                        }
                    })
                    .collect::<Vec<_>>(),
            }}
        </ul>
    }
}
