use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use common::response::search::SearchArticleItem;
use common::response::tag::TagListItem;

use crate::page::fetch::{LoadError, Loaded, notify_load_failures};
use crate::page::validation::validate_uuid;

#[component]
pub fn TagDetail() -> impl IntoView {
    let params = use_params_map();
    let tag_id = move || params.get().get("tag_id").unwrap_or_default();

    let detail: LocalResource<Loaded<(TagListItem, Vec<SearchArticleItem>)>> =
        LocalResource::new(move || {
            let id = tag_id();
            async move {
                validate_uuid(&id)?;
                let tag_view = crate::request::tag::read_tag(&id)
                    .await
                    .map_err(LoadError::from)?;
                let page = crate::request::article::search_articles(&[
                    ("q", tag_view.name.as_str()),
                    ("ranges", "tag"),
                    ("page", "1"),
                    ("limit", "8"),
                ])
                .await
                .map_err(LoadError::from)?;
                Ok((tag_view, page.items))
            }
        });
    notify_load_failures(detail);

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || match detail.get() {
                Some(Ok((tag_view, items))) => {
                    let tag_id = tag_view.id.clone();
                    let update_href = format!("/tag/{tag_id}/update");
                    let delete_href = format!("/tag/{tag_id}/delete");
                    let article_rows = if items.is_empty() {
                        view! { <p>"no articles"</p> }.into_any()
                    } else {
                        view! {
                            <ul>
                                {items.into_iter().map(|item| {
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
                                }).collect::<Vec<_>>()}
                            </ul>
                        }
                        .into_any()
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
                }
                Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                None => view! { <p>"Loading..."</p> }.into_any(),
            }}
        </Suspense>
    }
}
