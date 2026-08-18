use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::components::A;
use nail_common::response::comment::CommentView;

use crate::page::time_format::format_timestamp;

pub struct CommentViewContext {
    pub base_path: String,
    pub current_page: u64,
    pub authenticated: bool,
    pub max_chars: u64,
    pub posting: RwSignal<bool>,
}

pub const STYLE: &str = r#"
.cmt-section {
    font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
    max-width: 640px;
}
.cmt-form {
    display: flex;
    flex-direction: column;
    border: 1px solid #d1d9e0;
    border-radius: 8px;
    overflow: hidden;
    background: #ffffff;
    margin-bottom: 20px;
    transition: border-color .15s ease, box-shadow .15s ease;
}
.cmt-form:focus-within {
    border-color: #0969da;
    box-shadow: 0 0 0 3px rgba(9, 105, 218, .15);
}
.cmt-input {
    width: 100%;
    box-sizing: border-box;
    padding: 10px 12px;
    border: none;
    font: inherit;
    line-height: 1.5;
    resize: vertical;
    background: transparent;
    color: #1f2328;
}
.cmt-input:focus {
    outline: none;
}
.cmt-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    border-top: 1px solid #eaeef2;
    background: #f6f8fa;
}
.cmt-counter {
    font-size: 12px;
    color: #656d76;
    font-variant-numeric: tabular-nums;
}
.cmt-btn {
    padding: 6px 16px;
    border: 1px solid #d1d9e0;
    border-radius: 8px;
    background: #ffffff;
    color: #1f2328;
    cursor: pointer;
    font-size: 14px;
    line-height: 1.4;
    transition: background .15s ease, border-color .15s ease;
}
.cmt-btn:hover {
    background: #f6f8fa;
    border-color: #b6bfc9;
}
.cmt-btn:disabled {
    opacity: .5;
    cursor: not-allowed;
}
.cmt-btn-primary {
    background: #1f2328;
    color: #ffffff;
    border-color: #1f2328;
}
.cmt-btn-primary:hover {
    background: #32383f;
    border-color: #32383f;
}
.cmt-btn-danger {
    background: #d1242f;
    color: #ffffff;
    border-color: #d1242f;
}
.cmt-btn-danger:hover {
    background: #b6231c;
    border-color: #b6231c;
}
.cmt-list {
    list-style: none;
    margin: 0;
    padding: 0;
    border-bottom: 1px solid #eaeef2;
}
.cmt-item {
    padding: 14px 0;
    border-top: 1px solid #eaeef2;
}
.cmt-meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
}
.cmt-meta-link {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    text-decoration: none;
    color: #1f2328;
}
.cmt-meta-link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
    color: #0969da;
}
.cmt-meta-link:hover .cmt-time,
.cmt-meta-link:hover .cmt-count,
.cmt-meta-link:hover .cmt-seq,
.cmt-meta-link:hover .cmt-name {
    color: #0969da;
}
.cmt-seq,
.cmt-name,
.cmt-time,
.cmt-count {
    font-size: 12px;
    color: #656d76;
    font-variant-numeric: tabular-nums;
}
.cmt-body {
    margin: 6px 0 0;
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.6;
    color: #1f2328;
}
.cmt-context {
    background: #f6f8fa;
    border: 1px solid #d1d9e0;
    border-radius: 8px;
    padding: 12px 14px;
    margin-bottom: 16px;
}
.cmt-context .cmt-body {
    margin-top: 8px;
}
.cmt-empty,
.cmt-loading {
    color: #656d76;
    font-size: 14px;
    margin: 8px 0;
}
"#;

pub fn comment_form<F>(
    value: RwSignal<String>,
    posting: RwSignal<bool>,
    max_chars: u64,
    placeholder: &'static str,
    submit_label: &'static str,
    on_submit: F,
) -> impl IntoView
where
    F: Fn(SubmitEvent) + Clone + 'static,
{
    view! {
        <form class="cmt-form" on:submit=on_submit>
            <textarea
                class="cmt-input"
                rows="3"
                placeholder=placeholder
                prop:value=value
                on:input=move |event| value.set(event_target_value(&event))
            ></textarea>
            <div class="cmt-row">
                <span class="cmt-counter">
                    {move || format!("{}/{}", value.get().chars().count(), max_chars)}
                </span>
                <button class="cmt-btn cmt-btn-primary" type="submit" disabled=move || posting.get()>
                    {move || if posting.get() { "posting..." } else { submit_label }}
                </button>
            </div>
        </form>
    }
}

pub fn comment_rows(
    comments: &[CommentView],
    base: &str,
    start_seq: u64,
) -> Vec<leptos::prelude::AnyView> {
    comments
        .iter()
        .enumerate()
        .map(|(index, comment)| {
            let seq = start_seq + index as u64 + 1;
            let id = comment.id.clone();
            let user_name = comment.user_name.clone();
            let body = comment.content.clone();
            let time = format_timestamp(comment.created_at);
            let child_count = comment.child_count;
            let href = format!("{base}/comment/{id}");
            view! {
                <div class="cmt-item">
                    <A attr:class="cmt-meta-link" href=href>
                        <span class="cmt-seq">{format!("{seq})")}</span>
                        <span class="cmt-name">{user_name}</span>
                        <span class="cmt-time">{time}</span>
                        <span class="cmt-count">{format!("({child_count})")}</span>
                    </A>
                    <p class="cmt-body">{body}</p>
                </div>
            }
            .into_any()
        })
        .collect()
}

pub struct CommentLinks {
    pub update: Option<String>,
    pub delete: Option<String>,
    pub undelete: Option<String>,
}

pub fn context_card(comment: &CommentView, links: CommentLinks) -> impl IntoView {
    let time = format_timestamp(comment.created_at);
    let user_name = comment.user_name.clone();
    let body = comment.content.clone();
    view! {
        <div class="cmt-context">
            <div class="cmt-meta">
                <span class="cmt-name">{user_name}</span>
                <span class="cmt-time">{time}</span>
            </div>
            <p class="cmt-body">{body}</p>
            {links.update.map(|url| view! { <p class="cmt-body"><A href=url>update</A></p> })}
            {links.delete.map(|url| view! { <p class="cmt-body"><A href=url>delete</A></p> })}
            {links.undelete.map(|url| view! { <p class="cmt-body"><A href=url>undelete</A></p> })}
        </div>
    }
}
