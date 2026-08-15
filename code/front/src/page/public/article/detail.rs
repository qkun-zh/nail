use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use nail_common::response::article::ArticleView;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::session_gate::{SessionStatus, use_session_status};
use crate::page::time_format::format_timestamp;

#[component]
pub fn ArticleDetail() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let session_status = use_session_status();
    let article = RwSignal::new(None::<ArticleView>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::article::read_article(&article_id).await {
                Ok(view) => article.set(Some(view)),
                Err(request_error) => {
                    notify_error(&notifications, request_error.to_string());
                    error.set(Some(request_error.to_string()));
                }
            }
        });
    });

    let render = move || {
        if let Some(message) = error.get() {
            return view! { <p>{message}</p> }.into_any();
        }
        let Some(article) = article.get() else {
            return view! { <p>loading...</p> }.into_any();
        };
        let created_at = format_timestamp(article.created_at);
        let tags = article
            .tags
            .iter()
            .map(|tag| tag.name.clone())
            .collect::<Vec<_>>()
            .join(" ");
        let article_id = article.id.clone();
        let update_href = format!("/public/article/{article_id}/update");
        let delete_href = format!("/public/article/{article_id}/delete");
        let versions_href = format!("/public/article/{article_id}/version");
        let has_session = matches!(session_status.get(), SessionStatus::Authenticated(_));
        view! {
            <div>
                <hr/>
                <p>{"title: "}{article.title}</p>
                <hr/>
                <p>{"author: "}{article.author_name}</p>
                <hr/>
                <p>{"publish time: "}{created_at}</p>
                <hr/>
                <p>{"summary: "}{article.summary}</p>
                <hr/>
                {if tags.is_empty() {
                    ().into_any()
                } else {
                    view! { <p>{"tags: "}{tags}</p> }.into_any()
                }}
                <hr/>
                <div><A href=versions_href>version</A></div>
                <hr/>
                {if has_session {
                    view! {
                        <div><A href=update_href>update</A></div>
                        <hr/>
                        <div><A href=delete_href>delete</A></div>
                        <hr/>
                    }
                    .into_any()
                } else {
                    ().into_any()
                }}
            </div>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
