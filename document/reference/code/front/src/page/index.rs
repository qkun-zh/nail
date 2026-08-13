use crate::page::auth_gate::SessionGate;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Index() -> impl IntoView {
    view! {
        <SessionGate>
            <div><A href="/public">public</A></div>
            <div><A href="/private">private</A></div>
        </SessionGate>
    }
}
