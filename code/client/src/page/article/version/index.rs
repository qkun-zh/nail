use common::response::ListPage;
use common::response::version::VersionListItem;
use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::{query_signal, use_params_map};

use crate::infrastructure::limits::use_limits;
use crate::page::fetch::{LoadError, Loaded, notify_load_failures};
use crate::page::pagination::{LevelPagination, clamp_page_size};
use crate::page::validation::validate_uuid;

#[component]
pub fn VersionList() -> impl IntoView {
    let params = use_params_map();
    let limits = use_limits();
    let (page_signal, _set_page) = query_signal::<u64>("page");
    let current_page = Memo::new(move |_| page_signal.get().unwrap_or(1).max(1));

    let versions: LocalResource<Loaded<ListPage<VersionListItem>>> =
        LocalResource::new(move || {
            let article_id = params.get().get("article_id").unwrap_or_default();
            let limit = clamp_page_size(limits.get().search_page_size, 8);
            let page_value = current_page.get();
            async move {
                validate_uuid(&article_id)?;
                crate::request::version::read_versions(&article_id, page_value, limit)
                    .await
                    .map_err(LoadError::from)
            }
        });
    notify_load_failures(versions);

    view! {
        <div>
            <Suspense fallback=|| view! { <p>loading...</p> }>
                {move || match versions.get() {
                    Some(Ok(view)) => {
                        let article_id = params.get().get("article_id").unwrap_or_default();
                        let create_href = format!("/article/{article_id}/version/create");
                        let current_page = current_page.get();
                        let has_next = view.has_next;
                        let rows = view
                            .items
                            .into_iter()
                            .map(|version| {
                                let version_id = version.id.clone();
                                let detail_href =
                                    format!("/article/{article_id}/version/{version_id}");
                                view! {
                                    <div><A href=detail_href.clone()>{version.version}</A></div>
                                }
                            })
                            .collect_view();
                        let pagination = view! {
                            <LevelPagination
                                current=move || current_page
                                has_next=move || has_next
                                base_href=format!("/article/{article_id}/version")
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
                    Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                    None => view! { <p>loading...</p> }.into_any(),
                }}
            </Suspense>
        </div>
        <Outlet/>
    }
}
