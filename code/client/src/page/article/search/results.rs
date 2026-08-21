use leptos::prelude::*;
use leptos_router::components::A;

use common::response::search::SearchArticleItem;

use crate::page::pagination::PrevNext;

use super::versions::SearchVersions;

#[component]
pub(super) fn SearchResults(
    list: RwSignal<Vec<SearchArticleItem>>,
    loaded: RwSignal<bool>,
    q_filter: RwSignal<String>,
    current_page: RwSignal<u64>,
    has_next: RwSignal<bool>,
    on_go: Callback<u64>,
) -> impl IntoView {
    let has_prev = move || current_page.get() > 1;
    view! {
        <div>
            {move || {
                let list = list.get();
                let q = q_filter.get_untracked();
                if !loaded.get() {
                    return view! { <p>loading...</p> }.into_any();
                }
                if list.is_empty() {
                    return if q.trim().is_empty() {
                        view! { <p class="empty-hint">enter a query to search</p> }
                            .into_any()
                    } else {
                        view! { <p>none</p> }.into_any()
                    };
                }
                let rows = list
                    .into_iter()
                    .map(|article| {
                        let detail_url = format!("/article/{}", article.article_id);
                        let author_url = format!("/user/{}", article.author_id);
                        let title_html = article.title.clone();
                        let author_html = article.author_name.clone();
                        let time_text = article.time.clone();
                        let article_hits = article.article_hits.clone();
                        let versions = article.versions.clone();
                        view! {
                            <div class="article">
                                <div class="article-head">
                                    <A attr:class="label-chip" href=detail_url>
                                        <span class="dot"></span>
                                        {"article"}
                                    </A>
                                    <span class="title" inner_html=title_html></span>
                                    <span class="meta">
                                        <A attr:class="author-link" href=author_url>
                                            <span inner_html=author_html></span>
                                        </A>
                                        {format!(" · {time_text}")}
                                    </span>
                                </div>
                                <div class="hits">
                                    {article_hits
                                        .into_iter()
                                        .map(|hit| {
                                            let label = hit.label.clone();
                                            let snippet = hit.snippet.clone();
                                            view! {
                                                <div class="field-card">
                                                    <div class="field-label">
                                                        <span class="dot"></span>{label}
                                                    </div>
                                                    <div class="field-body" inner_html=snippet></div>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                    <SearchVersions
                                        article_id=article.article_id.clone()
                                        versions=versions
                                    />
                                </div>
                            </div>
                        }
                    })
                    .collect_view();
                view! {
                    <div>
                        {rows}
                        <PrevNext
                            current=move || current_page.get()
                            has_prev=has_prev
                            has_next=move || has_next.get()
                            on_go=on_go
                        />
                    </div>
                }
                .into_any()
            }}
        </div>
    }
    .into_any()
}
