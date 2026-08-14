use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use crate::infrastructure::limits::use_limits;
use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::validate_comment_content;

#[component]
pub fn CommentReply() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let query = use_query_map();
    let notifications = use_notifications();
    let limits = use_limits();
    let body = RwSignal::new(query.get_untracked().get("reply").unwrap_or_default());

    let article_id = params.get_untracked().get("article_id").unwrap_or_default();
    let version_id = params.get_untracked().get("version_id").unwrap_or_default();
    let parent_id = params.get_untracked().get("comment_id").unwrap_or_default();
    let comments_href = format!("/public/article/{article_id}/version/{version_id}/comment");

    let pathname = format!("/public/article/{article_id}/version/{version_id}/comment/{parent_id}");
    persist_draft(navigate.clone(), pathname, move || {
        vec![("reply", body.get())]
    });

    let on_submit = {
        let parent_id = parent_id.clone();
        let comments_href = comments_href.clone();
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        Callback::new(move |event: SubmitEvent| {
            event.prevent_default();
            let limits = limits.get();
            let content = match validate_comment_content(&body.get(), limits.max_comment_body_chars)
            {
                Ok(value) => value,
                Err(error) => {
                    notify_error(&notifications, &error);
                    return;
                }
            };
            let parent_id = parent_id.clone();
            let content = content.clone();
            let comments_href = comments_href.clone();
            let notifications = notifications.clone();
            let navigate = navigate.clone();
            leptos::task::spawn_local(async move {
                match crate::request::comment::create_reply(&parent_id, &content).await {
                    Ok(_) => {
                        notify_success(&notifications, "reply created");
                        navigate(
                            &comments_href,
                            NavigateOptions {
                                resolve: false,
                                ..Default::default()
                            },
                        );
                    }
                    Err(error) => notify_error(&notifications, error.to_string()),
                }
            });
        })
    };

    view! {
            <div>
                <p>reply to comment {parent_id.clone()}</p>
                <form on:submit=move |event| on_submit.run(event)>
                    <textarea prop:value=body on:input=move |event| body.set(event_target_value(&event))></textarea>
                    <button type="submit">reply</button>
                </form>
            </div>
    }
}
