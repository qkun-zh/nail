use common::response::ListPage;
use leptos::prelude::*;
use leptos_router::components::A;

use crate::infrastructure::limits::use_limits;
use crate::page::fetch::LoadError;
use crate::page::paged_links::PagedLinks;
use crate::request::tag::{self, TagListItem};

async fn load_tags(page: u64, limit: u64) -> Result<ListPage<TagListItem>, LoadError> {
    tag::read_tags(Some(page), Some(limit))
        .await
        .map_err(LoadError::from)
}

#[component]
pub fn TagList() -> impl IntoView {
    let limits = use_limits();
    let per_page = Signal::derive(move || limits.get().tag_page_size);
    view! {
        <PagedLinks
            per_page=per_page
            label="tags"
            empty_message="no tags yet"
            load=load_tags
            render=move |tag: &TagListItem| {
                let href = format!("/tag/{}", tag.id);
                let name = tag.name.clone();
                let count = tag.article_count;
                view! {
                    <div class="flex items-baseline justify-between gap-4">
                        <A
                            href=href
                            attr:class="text-ink decoration-ink/50 underline-offset-4 hover:decoration-ink"
                        >{name}</A>
                        <span class="text-sm text-muted">{format!("{count} articles")}</span>
                    </div>
                }
                .into_any()
            }
        />
        <div class="mt-6 text-center">
            <A href="/tag/create">create tag</A>
        </div>
    }
}
