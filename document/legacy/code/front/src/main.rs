mod conf;
mod limits;
mod page;
mod pow;
mod req;
mod router;

use leptos::prelude::*;
use page::notify::{NotifyDisplay, provide_notify};

#[allow(non_snake_case)]
#[component]
fn App() -> impl IntoView {
    let _handle = provide_notify();
    limits::provide_limits();
    view! {
        <NotifyDisplay/>
        <router::All/>
    }
}

fn main() {
    if web_sys::window().is_some() {
        console_error_panic_hook::set_once();
        leptos::mount::mount_to_body(App);
    }
}
