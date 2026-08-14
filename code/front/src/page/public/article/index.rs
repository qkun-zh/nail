use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};
use nail_common::response::article::ArticleListPage;
use nail_common::response::search::SearchPage;

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, use_notifications};
use crate::page::pagination::{clamp_page_size, pagination_state};

#[derive(Clone)]
enum ArticlePage {
    Loading,
    List(ArticleListPage),
    Search(SearchPage),
    Error(String),
}

#[component]
pub fn ArticleIndex() -> impl IntoView {
    let query = use_query_map();
    let navigate = use_navigate();
    let notifications = use_notifications();
    let limits = use_limits();
    let state = RwSignal::new(ArticlePage::Loading);
    let search_input = RwSignal::new(query.get_untracked().get("q").unwrap_or_default());

    Effect::new(move |_| {
        let params = query.get();
        let page_value = params
            .get("page")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1)
            .max(1);
        let limit = clamp_page_size(limits.get().search_page_size, 8);

        let search_fields = [
            params.get("q"),
            params.get("ranges"),
            params.get("sort"),
            params.get("from"),
            params.get("to"),
        ];
        let is_search = search_fields.iter().any(|field| field.is_some());

        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            if is_search {
                let mut owned: Vec<(String, String)> = vec![
                    ("page".to_string(), page_value.to_string()),
                    ("limit".to_string(), limit.to_string()),
                ];
                for (key, value) in [
                    ("q", params.get("q")),
                    ("ranges", params.get("ranges")),
                    ("sort", params.get("sort")),
                    ("from", params.get("from")),
                    ("to", params.get("to")),
                ] {
                    if let Some(value) = value {
                        owned.push((key.to_string(), value));
                    }
                }
                let borrows: Vec<(&str, &str)> = owned
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect();
                match crate::request::article::search_articles(&borrows).await {
                    Ok(page) => state.set(ArticlePage::Search(page)),
                    Err(error) => {
                        notify_error(&notifications, error.to_string());
                        state.set(ArticlePage::Error(error.to_string()));
                    }
                }
            } else {
                match crate::request::article::read_articles(page_value, limit).await {
                    Ok(page) => state.set(ArticlePage::List(page)),
                    Err(error) => {
                        notify_error(&notifications, error.to_string());
                        state.set(ArticlePage::Error(error.to_string()));
                    }
                }
            }
        });
    });

    let submit_search = move |event: SubmitEvent| {
        event.prevent_default();
        let value = search_input.get();
        let pathname = "/public/article".to_string();
        let fields: Vec<(&str, &str)> = if value.is_empty() {
            vec![]
        } else {
            vec![("q", value.as_str())]
        };
        navigate(
            &crate::page::draft::draft_url(&pathname, &fields),
            NavigateOptions {
                resolve: false,
                ..Default::default()
            },
        );
    };

    let render = move || match state.get() {
        ArticlePage::Loading => view! { <p>loading...</p> }.into_any(),
        ArticlePage::Error(message) => view! { <p>{message}</p> }.into_any(),
        ArticlePage::List(page) => {
            let pagination = pagination_state(page.page, page.has_next);
            let rows = page
                .article_list
                .into_iter()
                .map(|article| {
                    let article_id = article.id.clone();
                    let detail = format!("/public/article/{article_id}");
                    let tags = article
                        .tags
                        .iter()
                        .map(|tag| tag.name.clone())
                        .collect::<Vec<_>>()
                        .join(" ");
                    view! {
                        <div>
                            <A href=detail.clone()>{article.title}</A>
                            <p>{article.summary}</p>
                            <p>{article.author_name} | {tags} | {article.latest_version}</p>
                        </div>
                    }
                })
                .collect_view();
            let previous = pagination.previous_page.map(|previous| {
                let href = format!("/public/article?page={previous}");
                view! { <A href=href.clone()>previous</A> }.into_any()
            });
            let next = pagination.next_page.map(|next| {
                let href = format!("/public/article?page={next}");
                view! { <A href=href.clone()>next</A> }.into_any()
            });
            view! {
                <div>
                    {rows}
                    {previous}
                    {next}
                </div>
            }
            .into_any()
        }
        ArticlePage::Search(page) => {
            let pagination = pagination_state(page.page, page.has_next);
            let rows = page
                .article_list
                .into_iter()
                .map(|article| {
                    let article_id = article.id.clone();
                    let detail = format!("/public/article/{article_id}");
                    let hits = article
                        .hits
                        .iter()
                        .map(|hit| format!("{}: {}", hit.label, hit.snippet))
                        .collect::<Vec<_>>()
                        .join(" | ");
                    view! {
                        <div>
                            <A href=detail.clone()>{article.title}</A>
                            <p>{article.author} | {article.time}</p>
                            <p>{hits}</p>
                        </div>
                    }
                })
                .collect_view();
            let encoded_q =
                crate::request::url::encode_component(&query.get().get("q").unwrap_or_default());
            let previous = pagination.previous_page.map(|previous| {
                let href = format!("/public/article?q={encoded_q}&page={previous}");
                view! { <A href=href.clone()>previous</A> }.into_any()
            });
            let next = pagination.next_page.map(|next| {
                let href = format!("/public/article?q={encoded_q}&page={next}");
                view! { <A href=href.clone()>next</A> }.into_any()
            });
            view! {
                <div>
                    {rows}
                    {previous}
                    {next}
                </div>
            }
            .into_any()
        }
    };

    view! {
            <div>
            <form on:submit=submit_search>
                <input
                    type="text"
                    prop:value=search_input
                    on:input=move |event| search_input.set(event_target_value(&event))
                />
                <button type="submit">search</button>
            </form>
            <A href="/public/article/create">publish article</A>
            {render}
            </div>
    }
}
