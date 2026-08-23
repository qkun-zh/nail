use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::confirm::{ConfirmButton, use_confirm_action};
use crate::page::fetch::require_id;
use crate::page::notify::{notify_success, use_notifications};

#[component]
pub fn UndeleteSoftArticle() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let confirm = use_confirm_action(move || {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        async move {
            require_id(&article_id)?;
            crate::request::article::undelete_soft_article(&article_id).await?;
            notify_success(&notifications, "article restored");
            navigate(
                &format!("/article/{article_id}"),
                NavigateOptions::default(),
            );
            Ok(())
        }
    });

    view! {
        <h1>"Undelete Article"</h1>
        <p class="warning">"Restore the soft-deleted article and its subtree?"</p>
        <ConfirmButton handle=confirm label="Restore" busy_label="Restoring..."/>
    }
}
