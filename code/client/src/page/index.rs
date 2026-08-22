use leptos::prelude::*;
use leptos_router::components::A;

use crate::page::session_gate::{SessionStatus, use_session_status};

#[component]
pub fn Index() -> impl IntoView {
    let session = use_session_status();
    let hub_link = move || match session.get() {
        SessionStatus::Authenticated(view) => {
            let href = format!("/user/{}", view.id.unwrap_or_default());
            view! { <div><A href=href>"my hub"</A></div> }.into_any()
        }
        _ => ().into_any(),
    };
    view! {
        <div><A href="/search">search</A></div>
        <div><A href="/authenticate">authenticate</A></div>
        <hr/>
        <h1>"manage"</h1>
        <div><A href="/tag">tags</A></div>
        <div><A href="/role">roles</A></div>
        <div><A href="/user">users</A></div>
        <hr/>
        {hub_link}
    }
}
