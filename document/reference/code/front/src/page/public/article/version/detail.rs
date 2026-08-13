use crate::page::auth_gate::use_component_alive;
use crate::page::public::{CommentSection, DownloadLink};
use crate::page::time::format_iso8601;
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

#[component]
pub fn Version() -> impl IntoView {
    let params = use_params_map();
    let alive = use_component_alive();

    let version_id = RwSignal::new(String::new());
    let context_time = RwSignal::new(String::new());
    let context_note = RwSignal::new(String::new());
    let download_url = RwSignal::new(String::new());
    let loaded = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let request_seq = StoredValue::new(0u64);

    Effect::new(move |_| {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let url_version_id = params.get().get("version_id").unwrap_or_default();
        let comment_path = params.get().get("comment_path").unwrap_or_default();
        let my_seq = request_seq.get_value() + 1;
        request_seq.set_value(my_seq);
        version_id.set(String::new());
        context_time.set(String::new());
        context_note.set(String::new());
        download_url.set(String::new());
        error.set(None);
        loaded.set(false);
        if article_id.is_empty() || url_version_id.is_empty() {
            error.set(Some("missing article id or version id".to_string()));
            loaded.set(true);
            return;
        }
        spawn_local({
            let alive = alive.clone();
            async move {
                match crate::req::read_version_detail(&url_version_id, &article_id).await {
                    Ok(data) => {
                        if !alive.get_value() {
                            return;
                        }
                        if request_seq.get_value() != my_seq {
                            return;
                        }
                        if let Some(created_at) = data.get("created_at").and_then(|v| v.as_u64()) {
                            version_id.set(url_version_id.clone());
                            context_time.set(format_iso8601(created_at));
                            if let Some(note) = data.get("note").and_then(|v| v.as_str()) {
                                context_note.set(note.to_string());
                            }
                        } else {
                            error.set(Some("version not found".to_string()));
                        }
                    }
                    Err(e) => {
                        if !alive.get_value() {
                            return;
                        }
                        if request_seq.get_value() != my_seq {
                            return;
                        }
                        error.set(Some(format!("version not found: {e}")));
                    }
                }

                if !alive.get_value() {
                    return;
                }
                if version_id.get_untracked().is_empty() {
                    loaded.set(true);
                    return;
                }
                if !comment_path.is_empty() {
                    loaded.set(true);
                    return;
                }
                let api_base_url = crate::conf::AppConfig::load().api_base_url;
                let session_token =
                    LocalStorage::get::<String>(crate::req::SESSION_TOKEN_KEY).unwrap_or_default();
                if session_token.is_empty() {
                    error.set(Some("authenticate to download".to_string()));
                    loaded.set(true);
                    return;
                }
                match crate::req::mint_download_url(&session_token, &article_id, &url_version_id)
                    .await
                {
                    Ok(dl) => {
                        if !alive.get_value() {
                            return;
                        }
                        if dl.starts_with('/') && !dl.starts_with("//") {
                            download_url.set(format!("{api_base_url}{dl}"));
                        } else {
                            error.set(Some("invalid download url".to_string()));
                        }
                    }
                    Err(e) => {
                        if !alive.get_value() {
                            return;
                        }
                        error.set(Some(format!("download mint failed: {e}")));
                    }
                }
                if !alive.get_value() {
                    return;
                }
                loaded.set(true);
            }
        });
    });

    view! {
        {move || if !loaded.get() {
            view! { <p>loading...</p> }.into_any()
        } else if let Some(err) = error.get() {
            view! { <p>{err}</p> }.into_any()
        } else {
            let version_id_value = version_id.get();
            if version_id_value.is_empty() {
                ().into_any()
            } else if params.get().get("comment_path").unwrap_or_default().is_empty() {
                let article_id_value = params.get().get("article_id").unwrap_or_default();
                let comment_url = format!(
                    "/public/article/{article_id_value}/version/{version_id_value}/comment"
                );
                view! {
                    <hr/>
                    <div>{context_time.get()}</div>
                    <hr/>
                    {if context_note.get().is_empty() {
                        None
                    } else {
                        Some(view! { <div>{context_note.get()}</div> })
                    }}
                    <hr/>
                    {if download_url.get().is_empty() {
                        None
                    } else {
                        Some(view! { <DownloadLink url=download_url.get()/> }.into_any())
                    }}
                    <hr/>
                    <A href=comment_url>comment</A>
                    <hr/>
                }.into_any()
            } else {
                view! { <CommentSection version_id=version_id_value/> }.into_any()
            }
        }}
    }
}
