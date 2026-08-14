pub mod update;

use leptos::prelude::*;
use leptos_router::components::A;

use crate::page::session_gate::{SessionStatus, use_session_status};

#[component]
pub fn Name() -> impl IntoView {
    let status = use_session_status();
    view! {
        <div>
            {move || match status.get() {
                SessionStatus::Authenticated(view) => {
                    let name = view.name.unwrap_or_default();
                    let greeting = if name.is_empty() {
                        "hi!".to_string()
                    } else {
                        format!("hi, {name}!")
                    };
                    view! { <p>{greeting}</p> }.into_any()
                }
                _ => view! { <p>hi!</p> }.into_any(),
            }}
            <div><A href="/private/name/update">update</A></div>
        </div>
    }
}
