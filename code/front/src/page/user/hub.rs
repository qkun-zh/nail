use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

#[component]
pub fn UserHub() -> impl IntoView {
    let params = use_params_map();
    let uid = move || params.get().get("uid").unwrap_or_default();
    view! {
        <div><A href={format!("/user/{}/id", uid())}>id</A></div>
        <div><A href={format!("/user/{}/name", uid())}>name</A></div>
        <div><A href={format!("/user/{}/email", uid())}>email</A></div>
        <div><A href={format!("/user/{}/role", uid())}>role</A></div>
        <div><A href={format!("/user/{}/article", uid())}>article</A></div>
        <div><A href={format!("/user/{}/logout", uid())}>logout</A></div>
        <div><A href={format!("/user/{}/deregister", uid())}>deregister</A></div>
    }
}
