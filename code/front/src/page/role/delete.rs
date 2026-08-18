use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};
use nail_common::response::role::RoleNameView;

use crate::request::role::{self};

#[component]
pub fn DeleteRole() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();

    let role = RwSignal::new(None::<RoleNameView>);
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let role_id = params.get().get("role_id").unwrap_or_default();
        leptos::task::spawn_local(async move {
            match role::read_role(&role_id).await {
                Ok(view) => role.set(Some(RoleNameView {
                    id: view.id,
                    name: view.name,
                })),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    });

    let on_confirm = move |_| {
        if submitting.get() {
            return;
        }
        submitting.set(true);
        error.set(None);

        let role_id = params.get().get("role_id").unwrap_or_default();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            match role::delete_role(&role_id).await {
                Ok(_) => {
                    navigate("/role", NavigateOptions::default());
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                    submitting.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Delete Role"</h1>
        {move || role.get().map(|role_view| view! {
            <p>"Are you sure you want to delete role \"" {role_view.name} "\"?"</p>
            <p class="warning">"This cannot be undone."</p>
        })}
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <button on:click=on_confirm disabled=submitting>
            {move || if submitting.get() { "Deleting..." } else { "Delete" }}
        </button>
    }
}
