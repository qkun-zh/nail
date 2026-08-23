use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::confirm::{ConfirmButton, use_confirm_action};
use crate::page::fetch::{Loaded, notify_load_failures, require_id};
use crate::request::tag;

#[component]
pub fn DeleteTag() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();

    let tag_id = move || params.get().get("tag_id").unwrap_or_default();

    let tag_name: LocalResource<Loaded<String>> = LocalResource::new(move || {
        let id = tag_id();
        async move {
            require_id(&id)?;
            Ok(tag::read_tag(&id).await?.name)
        }
    });
    notify_load_failures(tag_name);

    let confirm = use_confirm_action(move || {
        let id = tag_id();
        let navigate = navigate.clone();
        async move {
            require_id(&id)?;
            tag::delete_tag(&id).await?;
            navigate("/tag", NavigateOptions::default());
            Ok(())
        }
    });

    view! {
        <h1>"Delete Tag"</h1>
        {move || match tag_name.get() {
            Some(Ok(name)) => view! {
                <p>"Are you sure you want to delete tag \"" {name} "\"?"</p>
                <p class="warning">"This will remove the tag from all articles."</p>
            }
            .into_any(),
            _ => ().into_any(),
        }}
        <ConfirmButton handle=confirm label="Delete" busy_label="Deleting..."/>
    }
}
