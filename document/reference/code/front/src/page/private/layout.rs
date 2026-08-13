use crate::page::auth_gate::{AUTHENTICATE_PATH, SessionGate};
use leptos::prelude::*;
use leptos_router::components::Outlet;

#[component]
pub fn PrivateLayout() -> impl IntoView {
    view! {
        <SessionGate always=AUTHENTICATE_PATH>
            <Outlet/>
        </SessionGate>
    }
}
