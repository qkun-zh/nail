use common::response::ListPage;
use common::response::comment::CommentView;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;

use crate::page::session_gate::who_are_you;

use super::pagination::COMMENTS_PER_PAGE;
use super::render::{CommentViewContext, comment_form, comment_rows};
use crate::page::pagination::LevelPagination;

pub fn comment_list_view(
    roots: &ListPage<CommentView>,
    comment_view_context: &CommentViewContext,
    body: RwSignal<String>,
    on_submit_comment: impl Fn(SubmitEvent) + Clone + 'static,
) -> impl IntoView {
    let has_next = roots.has_next;
    let rows = comment_rows(
        &roots.items,
        &comment_view_context.base_path,
        (comment_view_context.current_page - 1) * COMMENTS_PER_PAGE,
    );
    let form = if comment_view_context.authenticated {
        comment_form(
            body,
            comment_view_context.posting,
            comment_view_context.max_chars,
            "comment",
            "comment",
            on_submit_comment,
        )
        .into_any()
    } else {
        who_are_you()
    };
    let list_view = if rows.is_empty() {
        view! { <p class="cmt-empty">no comments yet</p> }.into_any()
    } else {
        view! { <div class="cmt-list">{rows}</div> }.into_any()
    };
    view! {
        <div>
            {form}
            {list_view}
            <LevelPagination
                current=comment_view_context.current_page
                has_next=has_next
                base_href=format!("{}/comment", comment_view_context.base_path)
            />
        </div>
    }
    .into_any()
}
