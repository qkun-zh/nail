use common::response::ListPage;
use common::response::comment::CommentView;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;

use crate::page::session_gate::who_are_you;

use super::pagination::COMMENTS_PER_PAGE;
use super::render::{CommentViewContext, comment_form, comment_rows, context_card};
use crate::page::pagination::LevelPagination;

pub fn comment_detail_view(
    target: &CommentView,
    children: &ListPage<CommentView>,
    comment_id: &str,
    comment_view_context: &CommentViewContext,
    reply_body: RwSignal<String>,
    on_submit_reply: impl Fn(SubmitEvent) + Clone + 'static,
) -> impl IntoView {
    let base_path = comment_view_context.base_path.clone();
    let links = super::render::CommentLinks {
        update: Some(format!("{base_path}/comment/{comment_id}/update")),
        delete: Some(format!("{base_path}/comment/{comment_id}/delete")),
        undelete: Some(format!("{base_path}/comment/{comment_id}/undelete-soft")),
    };
    let form = if comment_view_context.authenticated {
        comment_form(
            reply_body,
            comment_view_context.posting,
            comment_view_context.max_chars,
            "reply",
            "reply",
            on_submit_reply,
        )
        .into_any()
    } else {
        who_are_you()
    };
    let rows = comment_rows(
        &children.items,
        &comment_view_context.base_path,
        (comment_view_context.current_page - 1) * COMMENTS_PER_PAGE,
    );
    let has_next = children.has_next;
    let children_view = if rows.is_empty() {
        view! { <p class="cmt-empty">no replies yet</p> }.into_any()
    } else {
        view! { <div class="cmt-list">{rows}</div> }.into_any()
    };
    view! {
        <div>
            {context_card(target, links)}
            {form}
            {children_view}
            <LevelPagination
                current=comment_view_context.current_page
                has_next=has_next
                base_href=format!("{}/comment/{comment_id}", comment_view_context.base_path)
            />
        </div>
    }
    .into_any()
}
