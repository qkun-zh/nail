use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use nail_common::response::comment::{CommentListPage, CommentView};

use crate::page::session_gate::who_are_you;

use super::pagination::{COMMENTS_PER_PAGE, LevelPagination};
use super::render::{CommentViewContext, comment_form, comment_rows, context_card};

pub fn comment_detail_view(
    target: RwSignal<Option<CommentView>>,
    children: RwSignal<Option<CommentListPage>>,
    comment_id: &str,
    ctx: &CommentViewContext,
    reply_body: RwSignal<String>,
    on_submit_reply: impl Fn(SubmitEvent) + Clone + 'static,
) -> impl IntoView {
    let Some(comment) = target.get() else {
        return view! { <p class="cmt-empty">comment not found</p> }.into_any();
    };
    let delete_url = format!("{}/comment/{comment_id}/delete", ctx.base_path);
    let form = if ctx.authenticated {
        comment_form(
            reply_body,
            ctx.posting,
            ctx.max_chars,
            "reply",
            "reply",
            on_submit_reply,
        )
        .into_any()
    } else {
        who_are_you()
    };
    let child_list = children.get();
    let rows = child_list
        .as_ref()
        .map(|list| {
            comment_rows(
                &list.comments,
                &ctx.base_path,
                (ctx.current_page - 1) * COMMENTS_PER_PAGE,
            )
        })
        .unwrap_or_default();
    let has_next = child_list.as_ref().is_some_and(|list| list.has_next);
    let children_view = if rows.is_empty() {
        view! { <p class="cmt-empty">no replies yet</p> }.into_any()
    } else {
        view! { <div class="cmt-list">{rows}</div> }.into_any()
    };
    view! {
        <div>
            {context_card(&comment, Some(delete_url))}
            {form}
            {children_view}
            <LevelPagination
                current=ctx.current_page
                has_next=has_next
                base_href=format!("{}/comment/{comment_id}", ctx.base_path)
            />
        </div>
    }
    .into_any()
}
