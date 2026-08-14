use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn PublicIndex() -> impl IntoView {
    view! {
            <div>
                <p>public area</p>
                <A href="/public/article">browse articles</A>
            </div>
    }
}
