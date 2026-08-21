use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Index() -> impl IntoView {
    view! {
        <div><A href="/search">search</A></div>
        <div><A href="/authenticate">authenticate</A></div>
    }
}
