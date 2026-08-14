use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};
use nail_common::request::DeleteMode;

use crate::page::author_gate::{denied_view, use_author_gate};
use crate::page::notify::{notify_error, notify_success, use_notifications};

#[component]
pub fn DeleteArticle() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();

    let title = RwSignal::new(String::new());
    let loaded = RwSignal::new(false);
    let working = RwSignal::new(false);

    let article_id = move || params.get().get("article_id");
    let (denied, checked) = use_author_gate(article_id);

    let effect_notifications = notifications.clone();
    Effect::new(move |_| {
        let Some(id) = params.get().get("article_id") else {
            return;
        };
        let notifications = effect_notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::article::read_article(&id, false).await {
                Ok(view) => {
                    title.set(view.title);
                    loaded.set(true);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    });

    let delete_notifications = notifications.clone();
    let delete = move |mode: DeleteMode| {
        if working.get() {
            return;
        }
        let Some(id) = params.get().get("article_id") else {
            return;
        };
        working.set(true);
        let notifications = delete_notifications.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::article::delete_article(&id, mode).await;
            working.set(false);
            match result {
                Ok(_) => {
                    notify_success(&notifications, "article deleted");
                    navigate(
                        "/public/article",
                        NavigateOptions {
                            resolve: false,
                            ..Default::default()
                        },
                    );
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    };

    let render = move || {
        let on_transfer = {
            let delete = delete.clone();
            Callback::new(move |_| delete(DeleteMode::Transfer))
        };
        let on_hard = {
            let delete = delete.clone();
            Callback::new(move |_| delete(DeleteMode::Hard))
        };
        if denied.get() && checked.get() {
            return denied_view();
        }
        if !checked.get() || !loaded.get() {
            return view! { <p>loading...</p> }.into_any();
        }
        let title = title.get();
        view! {
            <div>
                <p>{title}</p>
                <div><button on:click=move |_| on_transfer.run(()) disabled=move || working.get()>{move || if working.get() { "deleting..." } else { "transfer" }}</button></div>
                <div><button on:click=move |_| on_hard.run(()) disabled=move || working.get()>{move || if working.get() { "deleting..." } else { "delete" }}</button></div>
            </div>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
