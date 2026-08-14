use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use nail_common::response::version::VersionListPage;

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, use_notifications};
use crate::page::pagination::{Pagination, clamp_page_size};

#[derive(Clone)]
enum VersionPage {
    Loading,
    Loaded(VersionListPage),
    Error(String),
}

#[component]
pub fn VersionList() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let limits = use_limits();
    let query = use_query_map();
    let navigate = use_navigate();
    let state = RwSignal::new(VersionPage::Loading);

    Effect::new(move |_| {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let limit = clamp_page_size(limits.get().search_page_size, 8);
        let page_value = query
            .get()
            .get("page")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1)
            .max(1);
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::version::read_versions(&article_id, page_value, limit).await {
                Ok(view) => state.set(VersionPage::Loaded(view)),
                Err(error) => {
                    notify_error(&notifications, error.to_string());
                    state.set(VersionPage::Error(error.to_string()));
                }
            }
        });
    });

    let render = move || match state.get() {
        VersionPage::Loading => view! { <p>loading...</p> }.into_any(),
        VersionPage::Error(message) => view! { <p>{message}</p> }.into_any(),
        VersionPage::Loaded(view) => {
            let article_id = params.get().get("article_id").unwrap_or_default();
            let create_href = format!("/public/article/{article_id}/version/create");
            let current_page = query
                .get()
                .get("page")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1)
                .max(1);
            let limit = clamp_page_size(limits.get().search_page_size, 8);
            let total_pages = view.total.div_ceil(limit);
            let has_next = view.has_next;
            let rows = view
                .version_list
                .into_iter()
                .map(|version| {
                    let version_id = version.id.clone();
                    let detail_href = format!("/public/article/{article_id}/version/{version_id}");
                    view! {
                        <div><A href=detail_href.clone()>{version.version}</A></div>
                    }
                })
                .collect_view();
            let navigate = navigate.clone();
            let on_go = Callback::new(move |target: u64| {
                navigate(
                    &format!("/public/article/{article_id}/version?page={target}"),
                    NavigateOptions {
                        resolve: false,
                        ..Default::default()
                    },
                );
            });
            let pagination = view! {
                <Pagination
                    current=move || current_page
                    total_pages=move || total_pages
                    has_more=move || has_next
                    on_go=on_go
                />
            };
            view! {
                <div>
                    <div><A href=create_href>create</A></div>
                    {rows}
                    {pagination}
                </div>
            }
            .into_any()
        }
    };

    view! { <div>{render}</div> }
}
