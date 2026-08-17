use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use nail_common::response::user::UserView;

use crate::page::notify::{notify_error, use_notifications};

#[component]
pub fn AdminUserDetail() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let user = RwSignal::new(None::<UserView>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let user_id = params.get().get("user_id").unwrap_or_default();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::user::read_user(&user_id).await {
                Ok(view) => user.set(Some(view)),
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
        let Some(user) = user.get() else {
            return view! { <p>loading...</p> }.into_any();
        };
        view! {
            <div>
                <h2>"User Detail"</h2>
                <hr/>
                <p>{"id: "}{user.id.unwrap_or_default()}</p>
                <hr/>
                <p>{"name: "}{user.name.unwrap_or_default()}</p>
                <hr/>
                <p>{"email hash: "}{user.email_hash.unwrap_or_default()}</p>
                <hr/>
            </div>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
