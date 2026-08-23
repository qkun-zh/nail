use common::response::article::ArticleView;
use leptos::prelude::*;
use leptos_router::components::{A, Outlet};
use leptos_router::hooks::use_params_map;

use crate::page::fetch::{LoadError, Loaded, notify_load_failures};
use crate::page::time_format::format_timestamp;
use crate::page::validation::validate_uuid;

#[component]
pub fn ArticleDetail() -> impl IntoView {
    let params = use_params_map();
    let article_id = move || params.get().get("article_id").unwrap_or_default();

    let article: LocalResource<Loaded<ArticleView>> = LocalResource::new(move || {
        let id = article_id();
        async move {
            validate_uuid(&id)?;
            crate::request::article::read_article(&id)
                .await
                .map_err(LoadError::from)
        }
    });
    notify_load_failures(article);

    view! {
        <div>
            <Suspense fallback=|| view! { <p>loading...</p> }>
                {move || match article.get() {
                    Some(Ok(article)) => article_view(article).into_any(),
                    Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                    None => view! { <p>loading...</p> }.into_any(),
                }}
            </Suspense>
        </div>
        <Outlet/>
    }
}

fn article_view(article: ArticleView) -> impl IntoView {
    let created_at = format_timestamp(article.created_at);
    let tags = article
        .tags
        .iter()
        .map(|tag| tag.name.clone())
        .collect::<Vec<_>>()
        .join(" ");
    let article_id = article.id.clone();
    let update_href = format!("/article/{article_id}/update");
    let delete_href = format!("/article/{article_id}/delete");
    let undelete_href = format!("/article/{article_id}/undelete-soft");
    let versions_href = format!("/article/{article_id}/version");
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
            <div>
                {article.tags.clone().into_iter().map(|tag| {
                    let tag_href = format!("/tag/{}", tag.id);
                    let apply_href = format!("/article/{}/tag/{}/apply", article_id, tag.id);
                    let unapply_href = format!("/article/{}/tag/{}/unapply", article_id, tag.id);
                    view! {
                        <div>
                            <span> <A href=tag_href>{tag.name}</A> </span>
                            <span> <A href=apply_href>apply</A> </span>
                            <span> <A href=unapply_href>unapply</A> </span>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
            <hr/>
            <div><A href=versions_href>version</A></div>
            <hr/>
            <div><A href=update_href>update</A></div>
            <hr/>
            <div><A href=delete_href>delete</A></div>
            <hr/>
            <div><A href=undelete_href>undelete</A></div>
            <hr/>
        </div>
    }
}
