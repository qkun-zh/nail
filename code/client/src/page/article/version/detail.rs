use common::response::version::VersionView;
use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::use_params_map;

use crate::page::confirm::use_confirm_action;
use crate::page::fetch::{LoadError, Loaded, notify_load_failures, require_id};
use crate::page::time_format::format_timestamp;

#[derive(Clone)]
struct VersionPage {
    view: VersionView,
    download: Result<String, String>,
}

#[component]
fn DownloadLink(url: String) -> impl IntoView {
    let href = url.clone();
    let handle = use_confirm_action(move || {
        let url = url.clone();
        async move {
            crate::request::download::download_pdf(&url).await?;
            Ok(())
        }
    });
    view! {
        <a
            href=href
            class:busy=handle.working
            on:click=move |event| {
                event.prevent_default();
                if !handle.working.get() {
                    handle.submit.run(());
                }
            }
        >
            {move || if handle.working.get() { "downloading..." } else { "download" }}
        </a>
    }
}

#[component]
pub fn VersionDetail() -> impl IntoView {
    let params = use_params_map();
    let article_id = move || params.get().get("article_id").unwrap_or_default();
    let version_id = move || params.get().get("version_id").unwrap_or_default();

    let detail: LocalResource<Loaded<VersionPage>> = LocalResource::new(move || {
        let article_id = article_id();
        let version_id = version_id();
        async move {
            let article_id = require_id(&article_id)?;
            let version_id = require_id(&version_id)?;
            let view = crate::request::version::read_version(&version_id, &article_id)
                .await
                .map_err(LoadError::from)?;
            let download = crate::request::download::mint_download_url(&article_id, &version_id)
                .await
                .map_err(|error| error.to_string());
            Ok(VersionPage { view, download })
        }
    });
    notify_load_failures(detail);

    view! {
        <div>
            <Suspense fallback=|| view! { <p>loading...</p> }>
                {move || match detail.get() {
                    Some(Ok(page)) => version_view(page, &article_id()).into_any(),
                    Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                    None => view! { <p>loading...</p> }.into_any(),
                }}
            </Suspense>
        </div>
        <Outlet/>
    }
}

fn version_view(page: VersionPage, article_id: &str) -> impl IntoView {
    let version = page.view;
    let created_at = format_timestamp(version.created_at);
    let comments_href = format!("/article/{article_id}/version/{}/comment", version.id);
    let update_href = format!("/article/{article_id}/version/{}/update", version.id);
    let delete_href = format!("/article/{article_id}/version/{}/delete", version.id);
    let undelete_href = format!("/article/{article_id}/version/{}/undelete-soft", version.id);
    let download = match page.download {
        Ok(url) => view! { <DownloadLink url/> }.into_any(),
        Err(message) => view! { <p>{message}</p> }.into_any(),
    };
    view! {
        <div>
            <hr/>
            <p>{version.version}</p>
            <hr/>
            <p>{created_at}</p>
            <hr/>
            {if version.note.is_empty() {
                ().into_any()
            } else {
                view! { <p>{version.note.clone()}</p> }.into_any()
            }}
            <hr/>
            {download}
            <hr/>
            <div><A href=comments_href>comment</A></div>
            <hr/>
            <div><A href=update_href>update</A></div>
            <hr/>
            <div><A href=delete_href>delete</A></div>
            <hr/>
            <div><A href=undelete_href>undelete</A></div>
            <hr/>
        </div>
    }
}
