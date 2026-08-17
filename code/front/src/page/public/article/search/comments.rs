use leptos::prelude::*;
use leptos_router::components::A;

use nail_common::response::search::SearchCommentItem;

use crate::page::pagination::LocalPagedList;

use super::super::version::comment::pagination::COMMENTS_PER_PAGE;

pub(super) fn render_comment(
    comment: &SearchCommentItem,
    article_id: &str,
    version_id: &str,
) -> impl IntoView {
    let comment_url = format!(
        "/public/article/{article_id}/version/{version_id}/comment/{}",
        comment.comment_id
    );
    let author_html = comment.author_name.clone();
    let time_text = comment.time.clone();
    let content_html = comment.content.clone();
    view! {
        <div class="comment-hit">
            <div class="comment-head-row">
                <div class="cmt-main">
                    <div class="cmt-meta">
                        <A attr:class="cmt-author" href=comment_url>
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
    let aid = article_id.clone();
    let vid = version_id.clone();
    let render = move |comment: &SearchCommentItem| render_comment(comment, &aid, &vid).into_any();
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
