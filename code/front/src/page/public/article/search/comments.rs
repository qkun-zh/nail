use leptos::prelude::*;
use leptos_router::components::A;

use nail_common::response::search::SearchCommentItem;

use crate::page::pagination::LocalPagedList;

use super::super::version::comment::pagination::COMMENTS_PER_PAGE;

pub(super) fn render_comment(
    comment: &SearchCommentItem,
    _article_id: &str,
    _version_id: &str,
) -> impl IntoView {
    let author_url = format!("/public/user/{}", comment.author_id);
    let author_html = comment.author_name.clone();
    let time_text = comment.time.clone();
    let content_html = comment.content.clone();
    view! {
        <div class="comment-hit">
            <div class="comment-head-row">
                <div class="cmt-main">
                    <div class="cmt-meta">
                        <A attr:class="cmt-author" href=author_url>
                            <span inner_html=author_html></span>
                            <span class="cmt-time">{time_text}</span>
                        </A>
                    </div>
                    <div class="cmt-content" inner_html=content_html></div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub(super) fn SearchComments(
    article_id: String,
    version_id: String,
    comments: Vec<SearchCommentItem>,
) -> impl IntoView {
    let render = move |comment: &SearchCommentItem| {
        render_comment(comment, &article_id, &version_id).into_any()
    };
    view! {
        <div class="field-card">
            <div class="field-label"><span class="dot"></span>comment</div>
            <div class="field-body">
                <LocalPagedList
                    items=comments
                    per_page=COMMENTS_PER_PAGE
                    pagination_class="comment-pagination"
                    render=render
                />
            </div>
        </div>
    }
}
