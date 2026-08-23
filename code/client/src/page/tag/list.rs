use leptos::prelude::*;
use leptos_router::components::A;

use common::response::ListPage;

use crate::page::fetch::{LoadError, Loaded, notify_load_failures};
use crate::request::tag::{self, TagListItem};

#[component]
pub fn TagList() -> impl IntoView {
    let tags: LocalResource<Loaded<ListPage<TagListItem>>> =
        LocalResource::new(
            || async move { tag::read_tags(None, None).await.map_err(LoadError::from) },
        );
    notify_load_failures(tags);

    view! {
        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            {move || match tags.get() {
                Some(Ok(page)) => view! {
                    <h1>"Tags"</h1>
                    <ul>
                        {page.items.into_iter().map(|tag| view! {
                            <li>
                                <A href={format!("/tag/{}", tag.id)}>
                                    {tag.name}
                                </A>
                                <span>" (" {tag.article_count} " articles)"</span>
                            </li>
                        }).collect::<Vec<_>>()}
                    </ul>
                    <p>"Total: " {page.total}</p>
                    <div><A href="/tag/create">create tag</A></div>
                }
                .into_any(),
                Some(Err(message)) => view! { <p>{message.to_string()}</p> }.into_any(),
                None => view! { <p>"Loading..."</p> }.into_any(),
            }}
        </Suspense>
    }
}
