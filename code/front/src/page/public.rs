pub mod article;
pub mod index;

use leptos::prelude::*;
use leptos_router::components::Outlet;

#[component]
pub fn PublicLayout() -> impl IntoView {
    view! { <Outlet/> }
}
