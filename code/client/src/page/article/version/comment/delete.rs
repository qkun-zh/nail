use common::request::DeleteMode;
use leptos::prelude::*;

use crate::page::delete_mode::{ALL_MODES, DeleteModePicker};

pub fn comment_delete_view(
    delete_mode: RwSignal<DeleteMode>,
    posting: RwSignal<bool>,
    on_submit_delete: impl Fn(DeleteMode) + Clone + 'static,
) -> impl IntoView {
    view! {
        <div>
            <p class="cmt-empty">confirm delete comment</p>
            <DeleteModePicker mode=delete_mode name="comment_delete_mode" allowed=&ALL_MODES/>
            <div>
                <button
                    class="cmt-btn cmt-btn-danger"
                    disabled=move || posting.get()
                    on:click=move |_| on_submit_delete(delete_mode.get())
                >
                    {move || if posting.get() { "deleting..." } else { "delete" }}
                </button>
            </div>
        </div>
    }
    .into_any()
}
