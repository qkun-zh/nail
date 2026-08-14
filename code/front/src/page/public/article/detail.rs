use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use nail_common::response::article::ArticleView;

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, use_notifications};
use crate::page::time_format::format_timestamp;

#[component]
pub fn ArticleDetail() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let limits = use_limits();
    let article = RwSignal::new(None::<ArticleView>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::article::read_article(&article_id, false).await {
                Ok(view) => article.set(Some(view)),
                Err(request_error) => {
                    notify_error(&notifications, &request_error.to_string());
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
        let created_at = format_timestamp(
            article.created_at,
            limits.get().timezone_offset_seconds,
        );
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
        view! {
            <div>
                <h2>{article.title}</h2>
                <p>{article.author_name} | {created_at}</p>
                <p>{tags}</p>
                <p>{article.summary}</p>
                <A href=versions_href>versions</A>
                <A href=update_href>update</A>
                <A href=delete_href>delete</A>
            </div>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
