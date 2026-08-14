use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};
use nail_common::request::DeleteMode;

use crate::page::notify::{notify_error, notify_success, use_notifications};

#[component]
pub fn CommentDelete() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let article_id = params.get_untracked().get("article_id").unwrap_or_default();
    let version_id = params.get_untracked().get("version_id").unwrap_or_default();
    let comment_id = params.get_untracked().get("comment_id").unwrap_or_default();
    let comments_href = format!("/public/article/{article_id}/version/{version_id}/comment");

    let delete = {
        let comment_id = comment_id.clone();
        move |mode: DeleteMode| {
            let comment_id = comment_id.clone();
            let comments_href = comments_href.clone();
            let notifications = notifications.clone();
            let navigate = navigate.clone();
            leptos::task::spawn_local(async move {
                match crate::request::comment::delete_comment(&comment_id, mode).await {
                    Ok(_) => {
                        notify_success(&notifications, "comment deleted");
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
        }
    };

    let on_transfer = {
        let delete = delete.clone();
        Callback::new(move |_| delete(DeleteMode::Transfer))
    };
    let on_hard = {
        let delete = delete.clone();
        Callback::new(move |_| delete(DeleteMode::Hard))
    };
    view! {
        <div>
            <div><button on:click=move |_| on_transfer.run(())>transfer</button></div>
            <div><button on:click=move |_| on_hard.run(())>delete</button></div>
        </div>
    }
}
