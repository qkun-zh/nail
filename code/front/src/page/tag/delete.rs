use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::request::tag::{self, TagNameView};

#[component]
pub fn DeleteTag() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();

    let tag = RwSignal::new(None::<TagNameView>);
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let tag_id = params.get().get("tag_id").unwrap_or_default();
        leptos::task::spawn_local(async move {
            match tag::read_tag(&tag_id).await {
                Ok(tag_view) => tag.set(Some(tag_view)),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    });

    let on_confirm = move |_| {
        submitting.set(true);
        error.set(None);

        let tag_id = params.get().get("tag_id").unwrap_or_default();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            match tag::delete_tag(&tag_id).await {
                Ok(()) => {
                    navigate("/tag", NavigateOptions::default());
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Delete Tag"</h1>
        {move || tag.get().map(|tag_view| view! {
            <p>"Are you sure you want to delete tag \"" {tag_view.name} "\"?"</p>
            <p class="warning">"This will remove the tag from all articles."</p>
        })}
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <button on:click=on_confirm disabled=submitting>
            {move || if submitting.get() { "Deleting..." } else { "Delete" }}
        </button>
    }
}
