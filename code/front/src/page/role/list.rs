use leptos::prelude::*;
use leptos_router::components::A;
use nail_common::response::ListPage;
use nail_common::response::role::RoleListItem;

use crate::request::role;

#[component]
pub fn RoleList() -> impl IntoView {
    let roles = RwSignal::new(None::<ListPage<RoleListItem>>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match role::read_roles(1, 200).await {
                Ok(page) => roles.set(Some(page)),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    });

    let render = move || {
        if let Some(message) = error.get() {
            return view! { <p>{message}</p> }.into_any();
        }
        let Some(page) = roles.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };
        view! {
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
        .into_any()
    };

    view! { {render} }
}
