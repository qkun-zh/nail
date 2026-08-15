pub mod update;

use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn EmailIndex() -> impl IntoView {
    view! {
        <div><A href="/private/email/update">update</A></div>
    }
}
