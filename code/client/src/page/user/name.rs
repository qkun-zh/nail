pub mod update;

use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::use_params_map;

use crate::page::fetch::{Loaded, notify_load_failures, require_id};

#[component]
pub fn Name() -> impl IntoView {
    let params = use_params_map();
    let uid = move || params.get().get("uid").unwrap_or_default();

    let name: LocalResource<Loaded<String>> = LocalResource::new(move || {
        let id = uid();
        async move {
            require_id(&id)?;
            Ok(crate::request::user::read_user(&id)
                .await?
                .name
                .unwrap_or_default())
        }
    });
    notify_load_failures(name);

    view! {
        <Suspense fallback=|| view! { <p>loading...</p> }>
            {move || match name.get() {
                Some(Ok(name)) => view! {
                    <p class="inline-block rounded-md border-4 border-ridge border-ink px-3 py-2 font-bold text-ink">{name}</p>
                }
                .into_any(),
                Some(Err(message)) => view! { <p class="font-bold text-ink">{message.to_string()}</p> }.into_any(),
                None => view! { <p>loading...</p> }.into_any(),
            }}
        </Suspense>
        <div><A href={format!("/user/{}/name/update", uid())}>update</A></div>
        <Outlet/>
    }
}
