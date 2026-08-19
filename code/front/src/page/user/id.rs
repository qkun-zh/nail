use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::page::validation::validate_uuid;

#[component]
pub fn UserId() -> impl IntoView {
    let params = use_params_map();
    let uid = params.get_untracked().get("uid").unwrap_or_default();
    match validate_uuid(&uid) {
        Err(message) => view! { <p>{message}</p> }.into_any(),
        Ok(uid) => view! { <p>{uid}</p> }.into_any(),
    }
}
