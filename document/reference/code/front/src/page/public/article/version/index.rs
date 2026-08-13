use crate::page::Pagination;
use crate::page::auth_gate::use_component_alive;
use crate::page::notify::{notify_error, use_notify};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::{use_location, use_navigate, use_params_map};

#[component]
pub fn VersionList() -> impl IntoView {
    let notification = use_notify();
    let params = use_params_map();
    let article_id = params.get_untracked().get("article_id").unwrap_or_default();
    let location = use_location();
    let navigate = use_navigate();
    let alive = use_component_alive();

    let versions = RwSignal::new(Vec::<serde_json::Value>::new());
    let loaded = RwSignal::new(false);
    let has_more = RwSignal::new(false);
    let total = RwSignal::new(0u64);
    let fetching = RwSignal::new(false);
    let current_page = RwSignal::new(1u64);
    let last_good_page = StoredValue::new(1u64);

    let page = location
        .query
        .get_untracked()
        .get("page")
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(1)
        .max(1);
    current_page.set(page);

    let pathname = location.pathname.get_untracked();

    let search_article_id = article_id.clone();
    let do_search = move |page: u64| {
        if fetching.get() {
            return;
        }
        if search_article_id.is_empty() {
            notify_error(&notification, "missing article id");
            loaded.set(true);
            return;
        }
        fetching.set(true);
        let aid = search_article_id.clone();
        spawn_local({
            let alive = alive.clone();
            async move {
                match crate::req::read_article_versions(&aid, page).await {
                    Ok(data) => {
                        if !alive.get_value() {
                            return;
                        }
                        if let Some(list) = data.get("version_list").and_then(|v| v.as_array()) {
                            versions.set(list.clone());
                        }
                        has_more.set(
                            data.get("has_next")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        );
                        total.set(data.get("total").and_then(|v| v.as_u64()).unwrap_or(0));
                        let pages = if total.get() == 0 {
                            1
                        } else {
                            total.get().div_ceil(8)
                        };
                        let committed = current_page.get().clamp(1, pages);
                        current_page.set(committed);
                        last_good_page.set_value(committed);
                    }
                    Err(e) => {
                        if !alive.get_value() {
                            return;
                        }
                        notify_error(&notification, &format!("load failed: {e}"));
                        current_page.set(last_good_page.get_value());
                    }
                }
                if !alive.get_value() {
                    return;
                }
                fetching.set(false);
                loaded.set(true);
            }
        });
    };

    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            navigate(
                &format!("{pathname}?page={}", current_page.get()),
                leptos_router::NavigateOptions {
                    replace: true,
                    resolve: false,
                    ..Default::default()
                },
            );
        }
    };

    Effect::new(move |_| {
        let _ = current_page.get();
        sync_url();
    });

    do_search.clone()(page);

    let on_go = Callback::new({
        let do_search = do_search.clone();
        move |page: u64| {
            let current_page_value = current_page.get();
            if page == current_page_value {
                return;
            }
            if fetching.get() {
                return;
            }
            current_page.set(page);
            do_search(page);
        }
    });

    let total_pages_on_page = move || {
        let total_value = total.get();
        if total_value == 0 {
            0
        } else {
            total_value.div_ceil(8)
        }
    };

    view! {
        {move || if !loaded.get() {
            view! { <p>loading...</p> }.into_any()
        } else {
            let list = versions.get();
            let new_url = format!(
                "/public/article/{}/version/create",
                crate::req::url_encode(&article_id)
            );
            view! {
                <div>
                    {list.into_iter().map(|entry| {
                        let version = entry.get("version").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let version_id = entry.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let version_url = format!(
                            "/public/article/{}/version/{}",
                            crate::req::url_encode(&article_id),
                            crate::req::url_encode(&version_id)
                        );
                        view! {
                            <div>
                                <A href=version_url>{version}</A>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
                <A href=new_url>create</A>
                <Pagination
                    current=move || current_page.get()
                    total_pages=move || total_pages_on_page()
                    has_more=move || has_more.get()
                    on_go=on_go
                />
            }.into_any()
        }}
    }
}
