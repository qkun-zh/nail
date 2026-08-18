pub mod update;

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

#[component]
pub fn EmailIndex() -> impl IntoView {
    let params = use_params_map();
    let uid = move || params.get().get("uid").unwrap_or_default();
    view! {
        <div><A href={format!("/user/{}/email/update", uid())}>update</A></div>
    }
}
