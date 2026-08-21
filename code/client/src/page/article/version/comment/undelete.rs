use leptos::prelude::*;

pub fn comment_undelete_view(
    posting: RwSignal<bool>,
    on_submit_undelete: impl Fn() + Clone + 'static,
) -> impl IntoView {
    view! {
        <div>
            <p class="cmt-empty">restore the soft-deleted comment?</p>
            <div>
                <button
                    class="cmt-btn cmt-btn-primary"
                    disabled=move || posting.get()
                    on:click=move |_| on_submit_undelete()
                >
                    {move || if posting.get() { "restoring..." } else { "restore" }}
                </button>
            </div>
        </div>
    }
    .into_any()
}
