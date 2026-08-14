use leptos::prelude::*;
use leptos_router::components::A;


#[component]
pub fn Index() -> impl IntoView {
    view! {
            <div>
                <p>nail</p>
                <A href="/public">public articles</A>
                <A href="/private">private area</A>
            </div>
    }
}
