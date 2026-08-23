use common::request::DeleteMode;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use crate::page::confirm::{ConfirmButton, use_confirm_action};
use crate::page::delete_mode::{DeleteModePicker, SOFT_AND_HARD, mode_from_str};
use crate::page::fetch::require_id;

#[component]
pub fn DeleteVersion() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let navigate = use_navigate();

    let mode = RwSignal::new(
        query
            .get_untracked()
            .get("mode")
            .and_then(|value| mode_from_str(&value, &SOFT_AND_HARD))
            .unwrap_or(DeleteMode::Soft),
    );

    let version_id = move || params.get().get("version_id").unwrap_or_default();
    let article_id = move || params.get().get("article_id").unwrap_or_default();

    let confirm = use_confirm_action(move || {
        let id = version_id();
        let article_id = article_id();
        let navigate = navigate.clone();
        async move {
            require_id(&id)?;
            require_id(&article_id)?;
            crate::request::version::delete_version(&id, mode.get()).await?;
            navigate(
                &format!("/article/{article_id}/version"),
                NavigateOptions::default(),
            );
            Ok(())
        }
    });

    view! {
        <h1>"Delete Version"</h1>
        <DeleteModePicker mode=mode name="version_delete_mode" allowed=&SOFT_AND_HARD/>
        <ConfirmButton handle=confirm label="delete" busy_label="deleting..."/>
    }
}
