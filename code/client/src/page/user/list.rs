use leptos::prelude::*;
use leptos_router::components::A;

use common::response::ListPage;
use common::response::user::UserListItem;

use crate::page::fetch::{LoadError, Loaded, notify_load_failures};
use crate::request::user;

#[component]
pub fn UserList() -> impl IntoView {
    let users: LocalResource<Loaded<ListPage<UserListItem>>> =
        LocalResource::new(
            || async move { user::read_users(1, 200).await.map_err(LoadError::from) },
        );
    notify_load_failures(users);

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || match users.get() {
                Some(Ok(page)) => view! {
                    <h1>"Users"</h1>
                    <ul>
                        {page.items.into_iter().map(|item| view! {
                            <li>
                                <A href={format!("/user/{}", item.id)}>
                                    {item.name}
                                </A>
                                <span>" (" {item.roles.len()} " roles)"</span>
                            </li>
                        }).collect::<Vec<_>>()}
                    </ul>
                    <p>"Total: " {page.total}</p>
                }
                .into_any(),
                Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                None => view! { <p>"Loading..."</p> }.into_any(),
            }}
        </Suspense>
    }
}
