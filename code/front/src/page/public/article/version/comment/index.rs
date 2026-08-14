use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use nail_common::response::comment::CommentListPage;

use crate::infrastructure::limits::use_limits;
use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::pagination::{clamp_page_size, pagination_state};
use crate::page::time_format::format_timestamp;
use crate::page::validation::validate_comment_content;

#[derive(Clone)]
enum CommentPage {
    Loading,
    Loaded(CommentListPage),
    Error(String),
}

#[component]
pub fn CommentIndex() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let query = use_query_map();
    let notifications = use_notifications();
    let limits = use_limits();
    let state = RwSignal::new(CommentPage::Loading);
    let body = RwSignal::new(query.get_untracked().get("body").unwrap_or_default());
    let posting = RwSignal::new(false);

    let version_id = move || params.get().get("version_id").unwrap_or_default();
    let article_id = move || params.get().get("article_id").unwrap_or_default();

    let pathname = format!(
        "/public/article/{}/version/{}/comment",
        params.get_untracked().get("article_id").unwrap_or_default(),
        params.get_untracked().get("version_id").unwrap_or_default()
    );
    persist_draft(navigate.clone(), pathname, move || {
        vec![("body", body.get())]
    });

    let reload = StoredValue::new({
        let notifications = notifications.clone();
        move |version_id: String| {
            let limit = clamp_page_size(limits.get().search_page_size, 8);
            let page_value = query
                .get()
                .get("page")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1)
                .max(1);
            let notifications = notifications.clone();
            leptos::task::spawn_local(async move {
                match crate::request::comment::read_comments(&version_id, page_value, limit).await {
                    Ok(view) => state.set(CommentPage::Loaded(view)),
                    Err(error) => {
                        notify_error(&notifications, error.to_string());
                        state.set(CommentPage::Error(error.to_string()));
                    }
                }
            });
        }
    });

    Effect::new(move |_| {
        let version_id = version_id();
        let reload = reload.get_value();
        reload(version_id);
    });

    let submit_notifications = notifications.clone();
    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        if posting.get() {
            return;
        }
        let version_id = version_id();
        let limits = limits.get();
        let content = match validate_comment_content(&body.get(), limits.max_comment_body_chars) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&submit_notifications, &error);
                return;
            }
        };
        posting.set(true);
        let notifications = submit_notifications.clone();
        let reload = reload.get_value();
        leptos::task::spawn_local(async move {
            let result = crate::request::comment::create_comment(&version_id, &content).await;
            posting.set(false);
            match result {
                Ok(_) => {
                    notify_success(&notifications, "comment created");
                    reload(version_id);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    };

    let render = move || {
        let submit = submit.clone();
        match state.get() {
            CommentPage::Loading => view! { <p>loading...</p> }.into_any(),
            CommentPage::Error(message) => view! { <p>{message}</p> }.into_any(),
            CommentPage::Loaded(view) => {
                let article_id = article_id();
                let version_id = version_id();
                let current_page = query
                    .get()
                    .get("page")
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(1)
                    .max(1);
                let pagination = pagination_state(current_page, view.has_next);
                let rows = view
                    .comments
                    .iter()
                    .enumerate()
                    .map(|(index, comment)| {
                        let seq = index as u64 + 1;
                        let id = comment.id.clone();
                        let reply_href = format!(
                            "/public/article/{article_id}/version/{version_id}/comment/{id}"
                        );
                        let delete_href = format!("{reply_href}/delete");
                        let created_at = format_timestamp(
                            comment.created_at,
                            limits.get().timezone_offset_seconds,
                        );
                        let indent = if comment.parent_id.is_some() {
                            "↳ "
                        } else {
                            ""
                        };
                        view! {
                            <div>
                                <p>{seq}{") "}{indent}{comment.user_name.clone()}{" · "}{created_at}</p>
                                <p>{comment.content.clone()}</p>
                                <div><A href=reply_href>reply</A></div>
                                <div><A href=delete_href>delete</A></div>
                            </div>
                        }
                    })
                    .collect_view();
                let previous = pagination.previous_page.map(|previous| {
                    let href = format!(
                        "/public/article/{article_id}/version/{version_id}/comment?page={previous}"
                    );
                    view! { <div><A href=href.clone()>previous</A></div> }.into_any()
                });
                let next = pagination.next_page.map(|next| {
                    let href = format!(
                        "/public/article/{article_id}/version/{version_id}/comment?page={next}"
                    );
                    view! { <div><A href=href.clone()>next</A></div> }.into_any()
                });
                view! {
                <div>
                    <form on:submit=submit>
                        <textarea placeholder="comment" prop:value=body on:input=move |event| body.set(event_target_value(&event))></textarea>
                        <span>{move || format!("{}/{}", body.get().chars().count(), limits.get().max_comment_body_chars)}</span>
                        <button type="submit" disabled=move || posting.get()>
                            {move || if posting.get() { "posting..." } else { "comment" }}
                        </button>
                    </form>
                    {rows}
                    {previous}
                    {next}
                </div>
            }
            .into_any()
            }
        }
    };

    view! { <div>{render}</div> }
}
