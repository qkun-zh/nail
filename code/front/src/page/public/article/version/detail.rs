use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use nail_common::response::version::VersionView;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::public::article::version::comment::CommentSection;
use crate::page::time_format::format_timestamp;

#[component]
fn DownloadLink(url: String) -> impl IntoView {
    let notifications = use_notifications();
    let downloading = RwSignal::new(false);
    let url_for_click = url.clone();
    let on_click = move |event: leptos::ev::MouseEvent| {
        event.prevent_default();
        if downloading.get() {
            return;
        }
        downloading.set(true);
        let url = url_for_click.clone();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            if let Err(message) = crate::request::download::download_pdf(&url).await {
                notify_error(&notifications, &message);
            }
            downloading.set(false);
        });
    };
    view! { <a href=url.clone() on:click=on_click>download</a> }
}

#[component]
pub fn VersionDetail() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let version = RwSignal::new(None::<VersionView>);
    let error = RwSignal::new(None::<String>);
    let download_url = RwSignal::new(None::<String>);
    let download_error = RwSignal::new(None::<String>);

    let effect_notifications = notifications.clone();
    Effect::new(move |_| {
        let comment_path = params.get().get("comment_path").unwrap_or_default();
        if !comment_path.is_empty() {
            return;
        }
        let version_id = params.get().get("version_id").unwrap_or_default();
        let article_id = params.get().get("article_id").unwrap_or_default();
        let notifications = effect_notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::version::read_version(&version_id, &article_id).await {
                Ok(view) => {
                    version.set(Some(view));
                    match crate::request::download::mint_download_url(&article_id, &version_id)
                        .await
                    {
                        Ok(minted_url) => download_url.set(Some(minted_url)),
                        Err(request_error) => {
                            notify_error(&notifications, request_error.to_string());
                            download_error.set(Some(request_error.to_string()));
                        }
                    }
                }
                Err(request_error) => {
                    notify_error(&notifications, request_error.to_string());
                    error.set(Some(request_error.to_string()));
                }
            }
        });
    });

    let render = move || {
        let comment_path = params.get().get("comment_path").unwrap_or_default();
        if !comment_path.is_empty() {
            return view! { <CommentSection/> }.into_any();
        }
        if let Some(message) = error.get() {
            return view! { <p>{message}</p> }.into_any();
        }
        let Some(version) = version.get() else {
            return view! { <p>loading...</p> }.into_any();
        };
        let created_at = format_timestamp(version.created_at);
        let article_id = params.get().get("article_id").unwrap_or_default();
        let comments_href = format!(
            "/public/article/{article_id}/version/{}/comment",
            version.id
        );
        let download = match download_url.get() {
            Some(url) => view! { <DownloadLink url=url/> }.into_any(),
            None => match download_error.get() {
                Some(message) => view! { <p>{message}</p> }.into_any(),
                None => view! { <p>loading...</p> }.into_any(),
            },
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
            </div>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
