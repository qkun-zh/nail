pub mod article;
pub mod index;
pub mod user;

use leptos::prelude::*;
use leptos_router::components::Outlet;

#[component]
pub fn PublicLayout() -> impl IntoView {
    view! { <Outlet/> }
}
