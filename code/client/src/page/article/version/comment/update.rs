use leptos::ev::SubmitEvent;
use leptos::prelude::*;

use super::render::comment_form;

pub fn comment_update_view(
    body: RwSignal<String>,
    posting: RwSignal<bool>,
    max_chars: u64,
    on_submit_update: impl Fn(SubmitEvent) + Clone + 'static,
) -> impl IntoView {
    view! {
        <div>
            <p class="cmt-empty">update comment</p>
            {comment_form(
                body,
                posting,
                max_chars,
                "content",
                "update",
                on_submit_update,
            )}
        </div>
    }
    .into_any()
}
