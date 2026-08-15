use crate::page::auth_gate::use_component_alive;
use crate::page::time::format_iso8601;
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

#[component]
pub fn Detail() -> impl IntoView {
    let params = use_params_map();
    let alive = use_component_alive();

    let article = RwSignal::new(None::<serde_json::Value>);
    let loaded = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let request_seq = StoredValue::new(0u64);

    Effect::new(move |_| {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let my_seq = request_seq.get_value() + 1;
        request_seq.set_value(my_seq);
        article.set(None);
        error.set(None);
        loaded.set(false);
        if article_id.is_empty() {
            error.set(Some("missing article id".to_string()));
            loaded.set(true);
            return;
        }
        spawn_local({
            let alive = alive.clone();
            async move {
                match crate::req::read_article_detail(&article_id).await {
                    Ok(data) => {
                        if !alive.get_value() {
                            return;
                        }
                        if request_seq.get_value() != my_seq {
                            return;
                        }
                        article.set(Some(data));
                        loaded.set(true);
                    }
                    Err(e) => {
                        if !alive.get_value() {
                            return;
                        }
                        if request_seq.get_value() != my_seq {
                            return;
                        }
                        let msg = format!("load failed: {e}");
                        if msg.contains("not found") {
                            loaded.set(true);
                        } else {
                            error.set(Some(msg));
                            loaded.set(true);
                        }
                    }
                }
            }
        });
    });

    view! {
        {move || if !loaded.get() {
            view! { <p>loading...</p> }.into_any()
        } else if let Some(err) = error.get() {
            view! { <p>{err}</p> }.into_any()
        } else if let Some(ref article_json) = article.get() {
            let title = article_json.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let summary = article_json.get("summary").and_then(|v| v.as_str()).unwrap_or("no summary");
            let id = article_json.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let author_name = article_json.get("author_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let created_at = article_json.get("created_at").and_then(|v| v.as_u64()).unwrap_or(0);
            let created_time = format_iso8601(created_at);
            let tags_list = article_json.get("tags").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let tag_names: Vec<String> = tags_list
                .into_iter()
                .filter_map(|tag_value| {
                    let tag_name = tag_value.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if tag_name.is_empty() { None } else { Some(tag_name) }
                })
                .collect();
            let versions_url = format!("/public/article/{}/version", crate::req::url_encode(id));
            let has_session = !LocalStorage::get::<String>(crate::req::SESSION_TOKEN_KEY)
                .unwrap_or_default()
                .is_empty();
            let update_url = format!("/public/article/{}/update", crate::req::url_encode(id));
            let delete_url = format!("/public/article/{}/delete", crate::req::url_encode(id));
            view! {
                <hr/>
                {if title.is_empty() { None } else { Some(view! { <div>{format!("title){title}")}</div> }) }}
                <hr/>
                {if author_name.is_empty() {
                    None
                } else {
                    Some(view! { <div>{format!("author){author_name}")}</div> })
                }}
                <hr/>
                {if created_time.is_empty() {
                    None
                } else {
                    Some(view! { <div>{format!("publish_time){created_time}")}</div> })
                }}
                <hr/>
                    <div>{format!("summary){summary}")}</div>
                <hr/>
                {if tag_names.is_empty() {
                    None
                } else {
                    Some(view! { <div>{format!("tag){}",tag_names.join(""))}</div> })
                }}
                <hr/>
                <A href=versions_url>version</A>
                <hr/>
                {if has_session {
                    view! {
                        <A href=update_url>update</A>
                        <hr/>
                        <A href=delete_url>delete</A>
                        <hr/>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            }.into_any()
        } else {
            view! { <p>not found</p> }.into_any()
        }}
    }
}
