use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::{query_signal, use_navigate, use_params_map};
use nail_common::response::ListPage;
use nail_common::response::version::VersionListItem;

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, use_notifications};
use crate::page::pagination::{PrevNext, clamp_page_size};
use crate::page::validation::validate_uuid;

#[derive(Clone)]
enum VersionPage {
    Loading,
    Loaded(ListPage<VersionListItem>),
    Error(String),
}

#[component]
pub fn VersionList() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let limits = use_limits();
    let navigate = use_navigate();
    let state = RwSignal::new(VersionPage::Loading);
    let (page_signal, _set_page) = query_signal::<u64>("page");
    let current_page = Memo::new(move |_| page_signal.get().unwrap_or(1).max(1));

    Effect::new(move |_| {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let limit = clamp_page_size(limits.get().search_page_size, 8);
        let page_value = current_page.get();
        let notifications = notifications.clone();
        if let Err(error_message) = validate_uuid(&article_id) {
            notify_error(&notifications, error_message.clone());
            state.set(VersionPage::Error(error_message));
            return;
        }
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
            let create_href = format!("/article/{article_id}/version/create");
            let current_page = current_page.get();
            let has_next = view.has_next;
            let has_prev = current_page > 1;
            let rows = view
                .items
                .into_iter()
                .map(|version| {
                    let version_id = version.id.clone();
                    let detail_href = format!("/article/{article_id}/version/{version_id}");
                    view! {
                        <div><A href=detail_href.clone()>{version.version}</A></div>
                    }
                })
                .collect_view();
            let navigate = navigate.clone();
            let on_go = Callback::new(move |target: u64| {
                navigate(
                    &format!("/article/{article_id}/version?page={target}"),
                    NavigateOptions {
                        resolve: false,
                        replace: true,
                        ..Default::default()
                    },
                );
            });
            let pagination = view! {
                <PrevNext
                    current=move || current_page
                    has_prev=move || has_prev
                    has_next=move || has_next
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

    view! {
        <div>{render}</div>
        <Outlet/>
    }
}
