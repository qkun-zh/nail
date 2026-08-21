use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::page::validation::validate_uuid;

#[component]
pub fn UserHub() -> impl IntoView {
    let params = use_params_map();
    let uid = params.get_untracked().get("uid").unwrap_or_default();
    match validate_uuid(&uid) {
        Err(message) => view! { <p>{message}</p> }.into_any(),
        Ok(uid) => view! {
            <div><A href={format!("/user/{uid}/id")}>id</A></div>
            <div><A href={format!("/user/{uid}/name")}>name</A></div>
            <div><A href={format!("/user/{uid}/email")}>email</A></div>
            <div><A href={format!("/user/{uid}/role")}>role</A></div>
            <div><A href={format!("/user/{uid}/article")}>article</A></div>
            <div><A href={format!("/user/{uid}/logout")}>logout</A></div>
            <div><A href={format!("/user/{uid}/deregister")}>deregister</A></div>
            <div><A href={format!("/user/{uid}/undelete-soft")}>undelete-soft</A></div>
        }
        .into_any(),
    }
}
