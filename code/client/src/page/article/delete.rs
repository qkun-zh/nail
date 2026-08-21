use common::request::DeleteMode;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use crate::page::author_gate::{denied_view, use_author_gate};
use crate::page::delete_mode::{ALL_MODES, DeleteModePicker, mode_from_str, mode_to_str};
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::validate_uuid;

#[component]
pub fn DeleteArticle() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let working = RwSignal::new(false);
    let mode = RwSignal::new(
        query
            .get_untracked()
            .get("mode")
            .and_then(|value| mode_from_str(&value, &ALL_MODES))
            .unwrap_or(DeleteMode::Transfer),
    );

    let article_id = move || params.get().get("article_id");
    let (denied, checked) = use_author_gate(article_id);

    crate::page::draft::sync_url_on_change(navigate.clone(), move || {
        let _ = mode.get();
        let id = params.get().get("article_id")?;
        Some(format!(
            "/article/{id}/delete?mode={}",
            mode_to_str(mode.get())
        ))
    });

    let delete_notifications = notifications.clone();
    let on_delete = Callback::new(move |()| {
        if working.get() {
            return;
        }
        let Some(id) = params.get().get("article_id") else {
            return;
        };
        if let Err(message) = validate_uuid(&id) {
            notify_error(&delete_notifications, message);
            working.set(false);
            return;
        }
        working.set(true);
        let notifications = delete_notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::article::delete_article(&id, mode.get()).await;
            working.set(false);
            match result {
                Ok(_) => notify_success(&notifications, "article deleted"),
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
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
                <div>
                    <button on:click=move |_| on_delete.run(()) disabled=move || working.get()>
                        {move || if working.get() { "deleting..." } else { "delete" }}
                    </button>
                </div>
            </div>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
