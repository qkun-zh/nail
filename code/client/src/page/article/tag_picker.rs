use leptos::prelude::*;

use common::response::tag::TagListItem;

use crate::page::fetch::{Loaded, notify_load_failures};
use crate::request::tag;

#[component]
pub fn TagPicker(selected: RwSignal<Vec<String>>) -> impl IntoView {
    let available_tags: LocalResource<Loaded<Vec<TagListItem>>> =
        LocalResource::new(|| async move { Ok(tag::read_tags(None, None).await?.items) });
    notify_load_failures(available_tags);

    let toggle_tag = move |tag_name: String| {
        let mut current = selected.get();
        if let Some(pos) = current.iter().position(|t| t == &tag_name) {
            current.remove(pos);
        } else {
            current.push(tag_name);
        }
        selected.set(current);
    };

    view! {
        <Suspense fallback=|| ().into_any()>
            {move || match available_tags.get() {
                Some(Ok(tags)) => view! {
                    <div class="tag-picker">
                        {tags.into_iter().map(|tag| {
                            let tag_name = tag.name.clone();
                            let is_checked = selected.get().contains(&tag_name);
                            view! {
                                <label class="tag-checkbox">
                                    <input
                                        type="checkbox"
                                        checked=is_checked
                                        on:change=move |_| toggle_tag(tag_name.clone())
                                    />
                                    {tag.name}
                                </label>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }
                .into_any(),
                Some(Err(_)) | None => ().into_any(),
            }}
        </Suspense>
    }
}
