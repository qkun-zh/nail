use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::request::tag::{self, TagNameView};

#[component]
pub fn UpdateTag() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();

    let tag = RwSignal::new(None::<TagNameView>);
    let name = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let tag_id = params.get().get("tag_id").unwrap_or_default();
        leptos::task::spawn_local(async move {
            match tag::read_tag(&tag_id).await {
                Ok(tag_view) => {
                    name.set(tag_view.name.clone());
                    tag.set(Some(tag_view));
                }
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        submitting.set(true);
        error.set(None);

        let tag_id = params.get().get("tag_id").unwrap_or_default();
        let name = name.get();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            match tag::update_tag(&tag_id, &name).await {
                Ok(_) => {
                    navigate(&format!("/tag/{tag_id}"), NavigateOptions::default());
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Update Tag"</h1>
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <form on:submit=on_submit>
            <label>
                "Name"
                <input
                    type="text"
                    prop:value=name
                    on:input=move |ev| name.set(event_target_value(&ev))
                    disabled=submitting
                />
            </label>
            <button type="submit" disabled=submitting>
                {move || if submitting.get() { "Updating..." } else { "Update" }}
            </button>
        </form>
    }
}
