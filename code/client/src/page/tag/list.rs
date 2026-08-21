use leptos::prelude::*;
use leptos_router::components::A;

use common::response::ListPage;

use crate::request::tag::{self, TagListItem};

#[component]
pub fn TagList() -> impl IntoView {
    let tags = RwSignal::new(None::<ListPage<TagListItem>>);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match tag::read_tags(None, None).await {
                Ok(page) => tags.set(Some(page)),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    });

    let render = move || {
        if let Some(message) = error.get() {
            return view! { <p>{message}</p> }.into_any();
        }
        let Some(page) = tags.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };
        view! {
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
        }
        .into_any()
    };

    view! { {render} }
}
