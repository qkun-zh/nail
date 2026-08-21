use common::request::DeleteMode;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use crate::page::delete_mode::{DeleteModePicker, SOFT_AND_HARD, mode_from_str};
use crate::page::validation::validate_uuid;

#[component]
pub fn DeleteVersion() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let navigate = use_navigate();

    let working = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let mode = RwSignal::new(
        query
            .get_untracked()
            .get("mode")
            .and_then(|value| mode_from_str(&value, &SOFT_AND_HARD))
            .unwrap_or(DeleteMode::Soft),
    );

    let version_id = move || params.get().get("version_id").unwrap_or_default();
    let article_id = move || params.get().get("article_id").unwrap_or_default();

    let on_delete = Callback::new(move |()| {
        if working.get() {
            return;
        }
        working.set(true);
        error.set(None);
        let version_id = version_id();
        let article_id = article_id();
        let navigate = navigate.clone();
        if let Err(message) = validate_uuid(&version_id).and_then(|_| validate_uuid(&article_id)) {
            error.set(Some(message));
            working.set(false);
            return;
        }
        leptos::task::spawn_local(async move {
            match crate::request::version::delete_version(&version_id, mode.get()).await {
                Ok(_) => {
                    navigate(
                        &format!("/article/{article_id}/version"),
                        NavigateOptions::default(),
                    );
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                    working.set(false);
                }
            }
        });
    });

    view! {
        <h1>"Delete Version"</h1>
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <DeleteModePicker mode=mode name="version_delete_mode" allowed=&SOFT_AND_HARD/>
        <button on:click=move |_| on_delete.run(()) disabled=move || working.get()>
            {move || if working.get() { "deleting..." } else { "delete" }}
        </button>
    }
}
