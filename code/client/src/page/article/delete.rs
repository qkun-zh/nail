use common::request::DeleteMode;
use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_query_map};

use crate::page::author_gate::{denied_view, use_author_gate};
use crate::page::confirm::{ConfirmButton, use_confirm_action};
use crate::page::delete_mode::{ALL_MODES, DeleteModePicker, mode_from_str, mode_to_str};
use crate::page::fetch::require_id;
use crate::page::notify::{notify_success, use_notifications};

#[component]
pub fn DeleteArticle() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let notifications = use_notifications();

    let mode = RwSignal::new(
        query
            .get_untracked()
            .get("mode")
            .and_then(|value| mode_from_str(&value, &ALL_MODES))
            .unwrap_or(DeleteMode::Transfer),
    );

    let article_id = move || params.get().get("article_id");
    let (denied, checked) = use_author_gate(article_id);

    crate::page::draft::mirror_param("mode", move || Some(mode_to_str(mode.get()).to_string()));

    let confirm = use_confirm_action(move || {
        let id = article_id().unwrap_or_default();
        let notifications = notifications.clone();
        async move {
            require_id(&id)?;
            crate::request::article::delete_article(&id, mode.get()).await?;
            notify_success(&notifications, "article deleted");
            Ok(())
        }
    });

    let render = move || {
        if denied.get() && checked.get() {
            return denied_view();
        }
        if !checked.get() {
            return view! { <p>loading...</p> }.into_any();
        }
        view! {
            <div>
                <DeleteModePicker mode=mode name="delete_mode" allowed=&ALL_MODES/>
                <ConfirmButton handle=confirm label="delete" busy_label="deleting..."/>
            </div>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
