use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Index() -> impl IntoView {
    view! {
        <div><A href="/private/email/check">check</A></div>
        <div><A href="/private/email/update">update</A></div>
    }
}
