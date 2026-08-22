use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::validation::validate_uuid;

#[component]
pub fn TagDetail() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let tag = RwSignal::new(None::<common::response::tag::TagListItem>);
    let articles = RwSignal::new(None::<Vec<common::response::search::SearchArticleItem>>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let tag_id = params.get().get("tag_id").unwrap_or_default();
        let notifications = notifications.clone();
        if let Err(error_message) = validate_uuid(&tag_id) {
            notify_error(&notifications, error_message.clone());
            error.set(Some(error_message));
            return;
        }
        leptos::task::spawn_local(async move {
            match crate::request::tag::read_tag(&tag_id).await {
                Ok(tag_view) => {
                    let tag_name = tag_view.name.clone();
                    tag.set(Some(tag_view));
                    match crate::request::article::search_articles(&[
                        ("q", &tag_name),
                        ("ranges", "tag"),
                        ("page", "1"),
                        ("limit", "8"),
                    ])
                    .await
                    {
                        Ok(page) => articles.set(Some(page.items)),
                        Err(request_error) => {
                            notify_error(&notifications, request_error.to_string());
                        }
                    }
                }
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
        let Some(tag_view) = tag.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };
        let tag_id = tag_view.id.clone();
        let update_href = format!("/tag/{tag_id}/update");
        let delete_href = format!("/tag/{tag_id}/delete");
        let article_rows = move || match articles.get() {
            None => view! { <p>"Loading articles..."</p> }.into_any(),
            Some(items) if items.is_empty() => view! { <p>"no articles"</p> }.into_any(),
            Some(items) => view! {
                <ul>
                    {items
                        .into_iter()
                        .map(|item| {
                            let href = format!("/article/{}", item.article_id);
                            view! {
                                <li>
                                    <A href=href>{item.title.clone()}</A>
                                    <span>" by "</span>
                                    <A href=format!(
                                        "/user/{}", item.author_id,
                                    )>{item.author_name.clone()}</A>
                                </li>
                            }
                        })
                        .collect::<Vec<_>>()}
                </ul>
            }
            .into_any(),
        };
        view! {
            <div>
                <h1>"Tag: " {tag_view.name.clone()}</h1>
                <p>"articles (" {tag_view.article_count} "):"</p>
                {article_rows}
                <hr/>
                <div><A href=update_href>"update"</A></div>
                <hr/>
                <div><A href=delete_href>"delete"</A></div>
                <hr/>
            </div>
        }
        .into_any()
    };

    view! { {render} }
}
