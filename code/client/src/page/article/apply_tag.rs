use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::confirm::{ConfirmButton, use_confirm_action};
use crate::page::fetch::require_id;
use crate::page::notify::{notify_success, use_notifications};

#[component]
pub fn ApplyTag() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let confirm = use_confirm_action(move || {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let tag_id = params.get().get("tag_id").unwrap_or_default();
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        async move {
            require_id(&article_id)?;
            require_id(&tag_id)?;
            crate::request::tag::apply_tag(&article_id, &tag_id).await?;
            notify_success(&notifications, "tag applied");
            navigate(
                &format!("/article/{article_id}"),
                NavigateOptions::default(),
            );
            Ok(())
        }
    });

    view! {
        <h1>"Apply Tag"</h1>
        <p>"Apply the tag to this article?"</p>
        <ConfirmButton handle=confirm label="Apply" busy_label="Applying..."/>
    }
}
