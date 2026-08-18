use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn UserId() -> impl IntoView {
    let params = use_params_map();
    let uid = move || params.get().get("uid").unwrap_or_default();
    view! {
        <p>{uid()}</p>
    }
}
