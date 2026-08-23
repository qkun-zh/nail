use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::confirm::{ConfirmButton, use_confirm_action};
use crate::page::fetch::require_id;
use crate::page::notify::{notify_success, use_notifications};

#[component]
pub fn UndeleteSoftUser() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let confirm = use_confirm_action(move || {
        let uid = params.get().get("uid").unwrap_or_default();
        let notifications = notifications.clone();
        let navigate = navigate.clone();
        async move {
            require_id(&uid)?;
            crate::request::user::undelete_soft_user(&uid).await?;
            notify_success(&notifications, "user restored");
            navigate(&format!("/user/{uid}"), NavigateOptions::default());
            Ok(())
        }
    });

    view! {
        <h1>"Undelete User"</h1>
        <p class="warning">"Restore the soft-deleted user account?"</p>
        <ConfirmButton handle=confirm label="Restore" busy_label="Restoring..."/>
    }
}
