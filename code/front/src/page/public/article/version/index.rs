use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_params_map, use_query_map};
use nail_common::response::version::VersionListPage;

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, use_notifications};
use crate::page::pagination::{clamp_page_size, pagination_state};
use crate::page::time_format::format_timestamp;

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
                    notify_error(&notifications, &error.to_string());
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
            let pagination = pagination_state(current_page, view.has_next);
            let rows = view
                .version_list
                .into_iter()
                .map(|version| {
                    let version_id = version.id.clone();
                    let detail_href = format!("/public/article/{article_id}/version/{version_id}");
                    let created_at =
                        format_timestamp(version.created_at, limits.get().timezone_offset_seconds);
                    view! {
                        <div>
                            <A href=detail_href.clone()>{version.version}</A>
                            <span>{created_at}</span>
                        </div>
                    }
                })
                .collect_view();
            let previous = pagination.previous_page.map(|previous| {
                let href = format!("/public/article/{article_id}/version?page={previous}");
                view! { <A href=href.clone()>previous</A> }.into_any()
            });
            let next = pagination.next_page.map(|next| {
                let href = format!("/public/article/{article_id}/version?page={next}");
                view! { <A href=href.clone()>next</A> }.into_any()
            });
            view! {
                <div>
                    <A href=create_href>add version</A>
                    {rows}
                    {previous}
                    {next}
                </div>
            }
            .into_any()
        }
    };

    view! { <div>{render}</div> }
}
