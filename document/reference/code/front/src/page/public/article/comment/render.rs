
use std::collections::HashMap;

use crate::limits::use_limits;
use crate::page::notify::{notify_error, use_notify};
use crate::page::time::format_iso8601;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

pub fn comment_id(comment: &serde_json::Value) -> String {
    comment
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn comment_user_name(comment: &serde_json::Value) -> String {
    comment
        .get("user_name")
        .and_then(|v| v.as_str())
        .unwrap_or("anonymous")
        .to_string()
}

pub fn comment_body(comment: &serde_json::Value) -> String {
    comment
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn comment_time(comment: &serde_json::Value) -> String {
    let created_at = comment
        .get("created_at")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    format_iso8601(created_at)
}

pub fn comment_form<F>(
    body: RwSignal<String>,
    posting: RwSignal<bool>,
    on_submit: F,
) -> leptos::prelude::AnyView
where
    F: Fn(SubmitEvent) + Clone + 'static,
{
    let limits = use_limits();
    view! {
        <form class="cmt-form" on:submit={on_submit.clone()}>
            <textarea
                class="cmt-input"
                rows="3"
                placeholder="comment"
                bind:value=body
            ></textarea>
            <div class="cmt-row">
                <span class="cmt-counter">
                    {move || {
                        let max_chars = limits.get().max_comment_body_chars;
                        format!("{}/{}", body.get().chars().count(), max_chars)
                    }}
                </span>
                <button class="cmt-btn cmt-btn-primary" type="submit" disabled=move || posting.get()>
                    {move || if posting.get() { "posting..." } else { "comment" }}
                </button>
            </div>
        </form>
    }
    .into_any()
}

pub fn render_comment_rows<'a>(
    comments: &[&'a serde_json::Value],
    children: &HashMap<&'a str, Vec<&'a serde_json::Value>>,
    base: &str,
) -> Vec<leptos::prelude::AnyView> {
    comments
        .iter()
        .enumerate()
        .map(|(idx, comment)| {
            let seq = idx as u32 + 1;
            let id = comment_id(comment);
            let user_name = comment_user_name(comment);
            let body = comment_body(comment);
            let time = comment_time(comment);
            let reply_count = children.get(id.as_str()).map(|v| v.len()).unwrap_or(0);
            let mut meta = vec![
                view! { <span class="cmt-seq">{format!("{seq})")}</span> }.into_any(),
                view! { <span class="cmt-name">{user_name}</span> }.into_any(),
            ];
            if !time.is_empty() {
                meta.push(view! { <span class="cmt-time">{time}</span> }.into_any());
            }
            meta.push(
                view! { <span class="cmt-count">{format!("({reply_count}")}</span> }.into_any(),
            );
            let meta_row = if id.is_empty() {
                view! { <div class="cmt-meta">{meta}</div> }.into_any()
            } else {
                view! {
                    <A attr:class="cmt-meta-link" href=format!("{base}/comment/{id}")>
                        {meta}
                    </A>
                }
                .into_any()
            };
            view! {
                <div class="cmt-item">
                    {meta_row}
                    <p class="cmt-body">{body}</p>
                </div>
            }
            .into_any()
        })
        .collect()
}

pub fn context_card(
    comment: &serde_json::Value,
    delete_url: Option<String>,
) -> leptos::prelude::AnyView {
    let user_name = comment_user_name(comment);
    let body = comment_body(comment);
    let time = comment_time(comment);
    view! {
        <div class="cmt-context">
            <div class="cmt-meta">
                <span class="cmt-name">{user_name}</span>
                {if time.is_empty() {
                    None
                } else {
                    Some(view! { <span class="cmt-time">{time}</span> })
                }}
            </div>
            <p class="cmt-body">{body}</p>
            {delete_url.map(|url| {
                view! { <p class="cmt-body"><A href=url>delete</A></p> }.into_any()
            })}
        </div>
    }
    .into_any()
}

pub fn delete_comment_form<F>(posting: RwSignal<bool>, on_submit: F) -> leptos::prelude::AnyView
where
    F: Fn(SubmitEvent) + Clone + 'static,
{
    view! {
        <form on:submit={on_submit.clone()}>
            <button type="submit" disabled=move || posting.get()>
                {move || if posting.get() { "deleting..." } else { "delete" }}
            </button>
        </form>
    }
    .into_any()
}

#[component]
pub fn DownloadLink(url: String) -> impl IntoView {
    let notification = use_notify();
    let downloading = RwSignal::new(false);
    let url_for_click = url.clone();
    let on_click = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        if downloading.get() {
            return;
        }
        downloading.set(true);
        let url = url_for_click.clone();
        spawn_local(async move {
            if let Err(e) = crate::req::download_pdf(&url).await {
                notify_error(&notification, &e);
            }
            downloading.set(false);
        });
    };
    view! {
        <a href=url.clone() on:click=on_click>download</a>
    }
}
