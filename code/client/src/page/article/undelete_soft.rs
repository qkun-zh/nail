use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::validate_uuid;

#[component]
pub fn UndeleteSoftArticle() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let working = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let on_confirm = move |_| {
        if working.get() {
            return;
        }
        working.set(true);
        error.set(None);

        let article_id = params.get().get("article_id").unwrap_or_default();
        let navigate = navigate.clone();
        let notifications = notifications.clone();
        if let Err(message) = validate_uuid(&article_id) {
            notify_error(&notifications, message.clone());
            error.set(Some(message));
            working.set(false);
            return;
        }
        leptos::task::spawn_local(async move {
            match crate::request::article::undelete_soft_article(&article_id).await {
                Ok(_) => {
                    notify_success(&notifications, "article restored");
                    navigate(
                        &format!("/article/{article_id}"),
                        NavigateOptions::default(),
                    );
                }
                Err(err) => {
                    notify_error(&notifications, err.to_string());
                    error.set(Some(err.to_string()));
                    working.set(false);
                }
            }
        });
    };

    view! {
        <h1>"Undelete Article"</h1>
        <p class="warning">"Restore the soft-deleted article and its subtree?"</p>
        {move || error.get().map(|err| view! { <p class="error">{err}</p> })}
        <button on:click=on_confirm disabled=working>
            {move || if working.get() { "Restoring..." } else { "Restore" }}
        </button>
    }
}
