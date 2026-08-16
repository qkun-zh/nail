use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use nail_common::response::comment::CommentListPage;

use crate::page::session_gate::who_are_you;

use super::pagination::{COMMENTS_PER_PAGE, LevelPagination};
use super::render::{CommentViewContext, comment_form, comment_rows};

pub fn comment_list_view(
    roots: RwSignal<Option<CommentListPage>>,
    ctx: &CommentViewContext,
    body: RwSignal<String>,
    on_submit_comment: impl Fn(SubmitEvent) + Clone + 'static,
) -> impl IntoView {
    let Some(list) = roots.get() else {
        return ().into_any();
    };
    let has_next = list.has_next;
    let rows = comment_rows(
        &list.comments,
        &ctx.base_path,
        (ctx.current_page - 1) * COMMENTS_PER_PAGE,
    );
    let form = if ctx.authenticated {
        comment_form(
            body,
            ctx.posting,
            ctx.max_chars,
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
                current=ctx.current_page
                has_next=has_next
                base_href=format!("{}/comment", ctx.base_path)
            />
        </div>
    }
    .into_any()
}
