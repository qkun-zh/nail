use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use nail_common::response::version::VersionView;

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::time_format::format_timestamp;

#[component]
pub fn VersionDetail() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let limits = use_limits();
    let version = RwSignal::new(None::<VersionView>);
    let error = RwSignal::new(None::<String>);
    let downloading = RwSignal::new(false);

    let effect_notifications = notifications.clone();
    Effect::new(move |_| {
        let version_id = params.get().get("version_id").unwrap_or_default();
        let article_id = params.get().get("article_id").unwrap_or_default();
        let notifications = effect_notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::version::read_version(&version_id, &article_id).await {
                Ok(view) => version.set(Some(view)),
                Err(request_error) => {
                    notify_error(&notifications, &request_error.to_string());
                    error.set(Some(request_error.to_string()));
                }
            }
        });
    });

    let download_notifications = notifications.clone();
    let download = move || {
        if downloading.get() {
            return;
        }
        downloading.set(true);
        let article_id = params.get().get("article_id").unwrap_or_default();
        let version_id = params.get().get("version_id").unwrap_or_default();
        let notifications = download_notifications.clone();
        leptos::task::spawn_local(async move {
            let result = match crate::request::download::mint_download_url(&article_id, &version_id).await {
                Ok(minted_url) => crate::request::download::download_pdf(&minted_url).await,
                Err(error) => Err(error.to_string()),
            };
            match result {
                Ok(()) => notify_success(&notifications, "download started"),
                Err(message) => notify_error(&notifications, &message),
            }
            downloading.set(false);
        });
    };

    let render = move || {
        let download = download.clone();
        if let Some(message) = error.get() {
            return view! { <p>{message}</p> }.into_any();
        }
        let Some(version) = version.get() else {
            return view! { <p>loading...</p> }.into_any();
        };
        let created_at = format_timestamp(
            version.created_at,
            limits.get().timezone_offset_seconds,
        );
        let article_id = params.get().get("article_id").unwrap_or_default();
        let comments_href = format!(
            "/public/article/{article_id}/version/{}/comment",
            version.id
        );
        view! {
            <div>
                <p>{version.version}</p>
                <p>{created_at}</p>
                <p>{version.note}</p>
                <button on:click=move |_| download()>download pdf</button>
                <A href=comments_href>comments</A>
            </div>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
