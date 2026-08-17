pub mod user;

use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use nail_common::response::user::UserListItem;

use crate::page::notify::{notify_error, use_notifications};

#[component]
pub fn AdminLayout() -> impl IntoView {
    view! { <Outlet/> }
}

#[component]
pub fn AdminIndex() -> impl IntoView {
    let notifications = use_notifications();
    let users = RwSignal::new(Vec::<UserListItem>::new());
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::user::read_users(&[("page", "1"), ("limit", "50")]).await {
                Ok(page) => users.set(page.user_list),
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
        let list = users.get();
        if list.is_empty() {
            return view! { <p>"no users"</p> }.into_any();
        }
        let rows = list
            .into_iter()
            .map(|user| {
                let href = format!("/admin/user/{}", user.id);
                let roles = user.roles.join(", ");
                view! {
                    <tr>
                        <td><A href=href>{user.id}</A></td>
                        <td>{user.name}</td>
                        <td>{roles}</td>
                    </tr>
                }
            })
            .collect::<Vec<_>>();
        view! {
            <table>
                <thead>
                    <tr>
                        <th>"id"</th>
                        <th>"name"</th>
                        <th>"roles"</th>
                    </tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>
        }
        .into_any()
    };

    view! {
        <div>
            <h2>"Users"</h2>
            <hr/>
            {render}
        </div>
    }
}
