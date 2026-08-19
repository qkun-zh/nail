use leptos::prelude::*;
use leptos_router::components::A;

use nail_common::response::ListPage;
use nail_common::response::user::UserListItem;

use crate::request::user;

#[component]
pub fn UserList() -> impl IntoView {
    let users = RwSignal::new(None::<ListPage<UserListItem>>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match user::read_users(1, 200).await {
                Ok(page) => users.set(Some(page)),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    });

    let render = move || {
        if let Some(message) = error.get() {
            return view! { <p>{message}</p> }.into_any();
        }
        let Some(page) = users.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };
        view! {
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
        .into_any()
    };

    view! { {render} }
}
