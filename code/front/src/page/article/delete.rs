use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use nail_common::request::DeleteMode;

use crate::page::author_gate::{denied_view, use_author_gate};
use crate::page::notify::{notify_error, notify_success, use_notifications};

fn mode_to_str(mode: DeleteMode) -> &'static str {
    match mode {
        DeleteMode::Transfer => "transfer",
        DeleteMode::Hard => "hard",
        DeleteMode::Soft => "soft",
    }
}

fn mode_from_str(value: &str) -> Option<DeleteMode> {
    match value {
        "transfer" => Some(DeleteMode::Transfer),
        "hard" => Some(DeleteMode::Hard),
        "soft" => Some(DeleteMode::Soft),
        _ => None,
    }
}

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
            .and_then(|value| mode_from_str(&value))
            .unwrap_or(DeleteMode::Transfer),
    );

    let article_id = move || params.get().get("article_id");
    let (denied, checked) = use_author_gate(article_id);

    let sync_url = {
        let navigate = navigate.clone();
        move || {
            let Some(id) = params.get().get("article_id") else {
                return;
            };
            navigate(
                &format!(
                    "/article/{id}/delete?mode={}",
                    mode_to_str(mode.get())
                ),
                NavigateOptions {
                    replace: true,
                    resolve: false,
                    ..Default::default()
                },
            );
        }
    };

    Effect::new(move |previous: Option<()>| {
        let _ = mode.get();
        if previous.is_none() {
            return;
        }
        sync_url();
    });

    let delete_notifications = notifications.clone();
    let on_delete = Callback::new(move |()| {
        if working.get() {
            return;
        }
        let Some(id) = params.get().get("article_id") else {
            return;
        };
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
        let is_transfer = move || mode.get() == DeleteMode::Transfer;
        let is_hard = move || mode.get() == DeleteMode::Hard;
        let is_soft = move || mode.get() == DeleteMode::Soft;
        view! {
            <div>
                <div>
                    <label>
                        <input type="radio" name="delete_mode" prop:checked=is_transfer on:change=move |_| mode.set(DeleteMode::Transfer)/>
                        "transfer"
                    </label>
                </div>
                <div>
                    <label>
                        <input type="radio" name="delete_mode" prop:checked=is_soft on:change=move |_| mode.set(DeleteMode::Soft)/>
                        "soft"
                    </label>
                </div>
                <div>
                    <label>
                        <input type="radio" name="delete_mode" prop:checked=is_hard on:change=move |_| mode.set(DeleteMode::Hard)/>
                        "hard"
                    </label>
                </div>
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
