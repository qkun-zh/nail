use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn ArticleIndex() -> impl IntoView {
    view! {
        <div><A href="/public/article/search">search</A></div>
        <div><A href="/public/article/create">create</A></div>
    }
}
