pub mod user;

use leptos::prelude::*;
use leptos_router::components::{A, Outlet};

#[component]
pub fn AdminLayout() -> impl IntoView {
    view! { <Outlet/> }
}

#[component]
pub fn AdminIndex() -> impl IntoView {
    view! {
        <div>
            <h2>"Admin"</h2>
            <hr/>
            <p>"Experimental admin section"</p>
            <hr/>
            <div><A href="/admin/user/1">"view user 1"</A></div>
        </div>
    }
}
