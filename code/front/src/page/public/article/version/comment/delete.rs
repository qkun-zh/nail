use leptos::prelude::*;
use nail_common::request::DeleteMode;

pub fn comment_delete_view(
    delete_mode: RwSignal<DeleteMode>,
    posting: RwSignal<bool>,
    on_submit_delete: impl Fn(DeleteMode) + Clone + 'static,
) -> impl IntoView {
    let is_transfer = move || delete_mode.get() == DeleteMode::Transfer;
    let is_soft = move || delete_mode.get() == DeleteMode::Soft;
    let is_hard = move || delete_mode.get() == DeleteMode::Hard;
    view! {
        <div>
            <p class="cmt-empty">confirm delete comment</p>
            <div>
                <label>
                    <input type="radio" name="comment_delete_mode" prop:checked=is_transfer on:change=move |_| delete_mode.set(DeleteMode::Transfer)/>
                    "transfer"
                </label>
            </div>
            <div>
                <label>
                    <input type="radio" name="comment_delete_mode" prop:checked=is_soft on:change=move |_| delete_mode.set(DeleteMode::Soft)/>
                    "soft"
                </label>
            </div>
            <div>
                <label>
                    <input type="radio" name="comment_delete_mode" prop:checked=is_hard on:change=move |_| delete_mode.set(DeleteMode::Hard)/>
                    "hard"
                </label>
            </div>
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
