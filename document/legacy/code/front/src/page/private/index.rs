use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn PrivateIndex() -> impl IntoView {
    view! {
        <div><A href="/private/authenticate">authenticate</A></div>
        <div><A href="/private/name">name</A></div>
        <div><A href="/private/email">email</A></div>
        <div><A href="/private/logout">logout</A></div>
        <div><A href="/private/deregister">deregister</A></div>
    }
}
