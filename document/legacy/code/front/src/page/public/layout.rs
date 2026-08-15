use crate::page::auth_gate::SessionGate;
use leptos::prelude::*;
use leptos_router::components::Outlet;

#[component]
pub fn PublicLayout() -> impl IntoView {
    view! {
        <SessionGate>
            <Outlet/>
        </SessionGate>
    }
}
