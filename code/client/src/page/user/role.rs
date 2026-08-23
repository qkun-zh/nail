use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use common::response::user::RoleRef;

use crate::page::fetch::{Loaded, notify_load_failures, require_id};

#[component]
pub fn UserRole() -> impl IntoView {
    let params = use_params_map();
    let uid = move || params.get().get("uid").unwrap_or_default();

    let roles: LocalResource<Loaded<Vec<RoleRef>>> = LocalResource::new(move || {
        let id = uid();
        async move {
            require_id(&id)?;
            Ok(crate::request::user::read_user(&id)
                .await?
                .roles
                .unwrap_or_default())
        }
    });
    notify_load_failures(roles);

    view! {
        <Suspense fallback=|| ().into_any()>
            {move || match roles.get() {
                Some(Ok(roles)) => view! {
                    <ul>
                        {roles.into_iter().map(|role| {
                            let name = role.name.clone();
                            view! {
                                <li>
                                    <A href={format!("/role/{}", role.id)}>{name}</A>
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                }
                .into_any(),
                Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                None => ().into_any(),
            }}
        </Suspense>
    }
}
