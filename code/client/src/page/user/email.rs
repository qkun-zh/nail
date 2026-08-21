pub mod update;

use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::use_params_map;

use crate::page::validation::validate_uuid;

#[component]
pub fn EmailIndex() -> impl IntoView {
    let params = use_params_map();
    let uid = params.get_untracked().get("uid").unwrap_or_default();
    match validate_uuid(&uid) {
        Err(message) => view! { <p>{message}</p> }.into_any(),
        Ok(uid) => view! {
            <div><A href={format!("/user/{uid}/email/update")}>update</A></div>
            <Outlet/>
        }
        .into_any(),
    }
}
