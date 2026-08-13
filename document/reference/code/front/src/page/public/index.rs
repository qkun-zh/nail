use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn PublicIndex() -> impl IntoView {
    view! { <A href="/public/article">article</A> }
}
