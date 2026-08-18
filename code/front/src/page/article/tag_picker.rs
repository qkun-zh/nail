use leptos::prelude::*;

use crate::request::tag::{self, TagListItem};

#[component]
pub fn TagPicker(selected: RwSignal<Vec<String>>) -> impl IntoView {
    let available_tags = RwSignal::new(Vec::<TagListItem>::new());
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match tag::read_tags(None, None).await {
                Ok(page) => available_tags.set(page.tag_list),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    });

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
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <div class="tag-picker">
            {move || available_tags.get().into_iter().map(|tag| {
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
}
