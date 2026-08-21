use common::response::role::RoleView;
use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::use_params_map;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::validation::validate_uuid;

#[component]
pub fn RoleDetail() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let role = RwSignal::new(None::<RoleView>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let role_id = params.get().get("role_id").unwrap_or_default();
        let notifications = notifications.clone();
        if let Err(error_message) = validate_uuid(&role_id) {
            notify_error(&notifications, error_message.clone());
            error.set(Some(error_message));
            return;
        }
        leptos::task::spawn_local(async move {
            match crate::request::role::read_role(&role_id).await {
                Ok(view) => role.set(Some(view)),
                Err(request_error) => {
                    notify_error(&notifications, request_error.to_string());
                    error.set(Some(request_error.to_string()));
                }
            }
        });
    });

    let render = move || {
        if let Some(message) = error.get() {
            return view! { <p>{message}</p> }.into_any();
        }
        let Some(role) = role.get() else {
            return view! { <p>loading...</p> }.into_any();
        };
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
    };

    view! {
        <div>{render}</div>
        <Outlet/>
    }
}
