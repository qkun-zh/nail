use common::response::role::RoleView;
use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::use_params_map;

use crate::page::fetch::{LoadError, Loaded, notify_load_failures};
use crate::page::validation::validate_uuid;

#[component]
pub fn RoleDetail() -> impl IntoView {
    let params = use_params_map();
    let role_id = move || params.get().get("role_id").unwrap_or_default();

    let role: LocalResource<Loaded<RoleView>> = LocalResource::new(move || {
        let id = role_id();
        async move {
            validate_uuid(&id)?;
            crate::request::role::read_role(&id)
                .await
                .map_err(LoadError::from)
        }
    });
    notify_load_failures(role);

    view! {
        <div>
            <Suspense fallback=|| view! { <p>loading...</p> }>
                {move || match role.get() {
                    Some(Ok(role)) => {
                        let role_id = role.id.clone();
                        let update_href = format!("/role/{role_id}/update");
                        let delete_href = format!("/role/{role_id}/delete");
                        view! {
                            <div>
                                <hr/>
                                <p>{"name: "}{role.name}</p>
                                <hr/>
                                <p>{"permissions ("}{role.permissions.len()}{"):"}</p>
                                <ul>
                                    {role.permissions.into_iter().map(|permission| view! {
                                        <li>{permission}</li>
                                    }).collect::<Vec<_>>()}
                                </ul>
                                <hr/>
                                <p>{"members ("}{role.members.len()}{"):"}</p>
                                <ul>
                                    {role.members.into_iter().map(|member| view! {
                                        <li><A href={format!("/user/{member}")}>{member}</A></li>
                                    }).collect::<Vec<_>>()}
                                </ul>
                                <hr/>
                                <div><A href=update_href>update</A></div>
                                <hr/>
                                <div><A href=delete_href>delete</A></div>
                                <hr/>
                            </div>
                        }
                        .into_any()
                    }
                    Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                    None => view! { <p>loading...</p> }.into_any(),
                }}
            </Suspense>
        </div>
        <Outlet/>
    }
}
