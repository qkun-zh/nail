use common::response::ListPage;
use common::response::role::RoleListItem;
use leptos::prelude::*;
use leptos_router::components::A;

use crate::page::fetch::{LoadError, Loaded, notify_load_failures};
use crate::request::role;

#[component]
pub fn RoleList() -> impl IntoView {
    let roles: LocalResource<Loaded<ListPage<RoleListItem>>> =
        LocalResource::new(
            || async move { role::read_roles(1, 200).await.map_err(LoadError::from) },
        );
    notify_load_failures(roles);

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || match roles.get() {
                Some(Ok(page)) => view! {
                    <h1>"Roles"</h1>
                    <ul>
                        {page.items.into_iter().map(|item| view! {
                            <li>
                                <A href={format!("/role/{}", item.id)}>
                                    {item.name}
                                </A>
                                <span>" (" {item.member_count} " members, " {item.permissions.len()} " permissions)"</span>
                            </li>
                        }).collect::<Vec<_>>()}
                    </ul>
                    <p>"Total: " {page.total}</p>
                    <div><A href="/role/create">create role</A></div>
                }
                .into_any(),
                Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                None => view! { <p>"Loading..."</p> }.into_any(),
            }}
        </Suspense>
    }
}
