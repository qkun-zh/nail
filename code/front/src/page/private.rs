pub mod authenticate;
pub mod deregister;
pub mod email;
pub mod index;
pub mod logout;
pub mod name;

use leptos::prelude::*;
use leptos_router::components::Outlet;

#[component]
pub fn PrivateLayout() -> impl IntoView {
    view! { <Outlet/> }
}
