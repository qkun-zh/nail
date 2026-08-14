pub mod update;

use leptos::prelude::*;
use leptos_router::components::A;

use crate::page::session_gate::{SessionStatus, use_session_status};

#[component]
pub fn Name() -> impl IntoView {
    let status = use_session_status();
    view! {
            <div>
                <p>name</p>
                {move || match status.get() {
                    SessionStatus::Authenticated(view) => view! { <p>{view.name.unwrap_or_default()}</p> }.into_any(),
                    _ => view! { <p>unknown</p> }.into_any(),
                }}
                <A href="/private/name/update">update name</A>
            </div>
    }
}
