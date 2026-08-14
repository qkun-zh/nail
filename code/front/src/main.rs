mod infrastructure;
mod page;
mod request;
mod router;

use leptos::mount::mount_to_body;
use leptos::prelude::*;

fn main() {
    // fail fast on an invalid compile-time api_base_url scheme (README 10)
    let _ = infrastructure::config::api_base_url();
    mount_to_body(|| {
        infrastructure::limits::provide_limits();
        page::notify::provide_notifications();
        page::session_gate::provide_session_state();
        request::session::set_session_invalid_hook(page::session_gate::mark_session_invalid);
        view! {
            <div>
                <router::AppRouter/>
                <page::notify::ToastContainer/>
            </div>
        }
    });
}
