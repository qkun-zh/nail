use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::confirm::{ConfirmButton, use_confirm_action};
use crate::page::fetch::{Loaded, notify_load_failures, require_id};
use crate::request::role::{self};

#[component]
pub fn DeleteRole() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();

    let role_id = move || params.get().get("role_id").unwrap_or_default();

    let role_name: LocalResource<Loaded<String>> = LocalResource::new(move || {
        let id = role_id();
        async move {
            require_id(&id)?;
            Ok(role::read_role(&id).await?.name)
        }
    });
    notify_load_failures(role_name);

    let confirm = use_confirm_action(move || {
        let id = role_id();
        let navigate = navigate.clone();
        async move {
            require_id(&id)?;
            role::delete_role(&id).await?;
            navigate("/role", NavigateOptions::default());
            Ok(())
        }
    });

    view! {
        <h1>"Delete Role"</h1>
        {move || match role_name.get() {
            Some(Ok(name)) => view! {
                <p>"Are you sure you want to delete role \"" {name} "\"?"</p>
                <p class="warning">"This cannot be undone."</p>
            }
            .into_any(),
            _ => ().into_any(),
        }}
        <ConfirmButton handle=confirm label="Delete" busy_label="Deleting..."/>
    }
}
