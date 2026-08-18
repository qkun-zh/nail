use leptos::prelude::*;
use leptos_router::components::A;

use nail_common::response::search::SearchVersionItem;

use crate::page::pagination::LocalPagedList;

use super::comments::SearchComments;

const VERSIONS_PER_PAGE: u64 = 8;

#[component]
pub(super) fn SearchVersions(
    article_id: String,
    versions: Vec<SearchVersionItem>,
) -> impl IntoView {
    let render = move |version: &SearchVersionItem| {
        let version_url = format!("/article/{}/version/{}", article_id, version.version_id);
        let version_chip_html = version.version_number.clone();
        let version_time_text = version.time.clone();
        let version_hits = version.version_hits.clone();
        let comments = version.comments.clone();
        let article_id_for_comments = article_id.clone();
        let version_id_for_comments = version.version_id.clone();
        let show_comments = !comments.is_empty();
        view! {
            <div class="field-card">
                <div class="field-label">
                    <span class="dot"></span>
                    <A attr:class="version-link" href=version_url>version</A>
                    <span class="version-chip" inner_html=version_chip_html></span>
                    <span class="version-time">{version_time_text}</span>
                </div>
                <div class="field-body">
                    {version_hits
                        .into_iter()
                        .map(|hit| {
                            let label = hit.label.clone();
                            let snippet = hit.snippet.clone();
                            view! {
                                <div class="field-card">
                                    <div class="field-label"><span class="dot"></span>{label}</div>
                                    <div class="field-body" inner_html=snippet></div>
                                </div>
                            }
                        })
                        .collect_view()}
                    {show_comments
                        .then(|| {
                            view! {
                                <SearchComments
                                    article_id=article_id_for_comments
                                    version_id=version_id_for_comments
                                    comments=comments
                                />
                            }
                        })}
                </div>
            </div>
        }
        .into_any()
    };
    view! {
        <LocalPagedList
            items=versions
            per_page=VERSIONS_PER_PAGE
            pagination_class="comment-pagination"
            render=render
        />
    }
}
