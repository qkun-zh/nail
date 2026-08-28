pub mod update;

use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::use_params_map;

use crate::page::fetch::{Loaded, notify_load_failures, require_id};

#[component]
pub fn EmailIndex() -> impl IntoView {
    let params = use_params_map();
    let uid = move || params.get().get("uid").unwrap_or_default();

    let email: LocalResource<Loaded<Option<String>>> = LocalResource::new(move || {
        let id = uid();
        async move {
            require_id(&id)?;
            let view = crate::request::user::read_user(&id).await?;
            Ok(view.email_hash)
        }
    });
    notify_load_failures(email);

    view! {
        <Suspense fallback=|| view! { <p>loading...</p> }>
            {move || match email.get() {
                Some(Ok(email_hash)) => view! { <p>{email_hash.unwrap_or_default()}</p> }.into_any(),
                Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                None => view! { <p>loading...</p> }.into_any(),
            }}
        </Suspense>
        <div><A href={format!("/user/{}/email/update", uid())}>update</A></div>
        <Outlet/>
    }
}
