use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use common::response::article::ArticleListItem;

use crate::page::fetch::{Loaded, notify_load_failures, require_id};
use crate::page::time_format::format_timestamp;

#[component]
pub fn UserArticle() -> impl IntoView {
    let params = use_params_map();
    let uid = move || params.get().get("uid").unwrap_or_default();

    let articles: LocalResource<Loaded<Vec<ArticleListItem>>> = LocalResource::new(move || {
        let id = uid();
        async move {
            require_id(&id)?;
            Ok(crate::request::user::read_user(&id)
                .await?
                .articles
                .unwrap_or_default())
        }
    });
    notify_load_failures(articles);

    view! {
        <div>
            <div><A href="/article/create">create article</A></div>
            <hr/>
            <Suspense fallback=|| view! { <p>"loading..."</p> }>
                {move || match articles.get() {
                    Some(Ok(list)) if list.is_empty() => view! { <p>"no articles"</p> }.into_any(),
                    Some(Ok(list)) => {
                        let rows = list
                            .into_iter()
                            .map(|a| {
                                let href = format!("/article/{}", a.id);
                                let created_at = format_timestamp(a.created_at);
                                view! {
                                    <div>
                                        <A href=href>{a.title}</A>
                                        <span>" "</span>
                                        <span>{created_at}</span>
                                    </div>
                                }
                            })
                            .collect_view();
                        view! { <div>{rows}</div> }.into_any()
                    }
                    Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                    None => view! { <p>"loading..."</p> }.into_any(),
                }}
            </Suspense>
        </div>
    }
}
