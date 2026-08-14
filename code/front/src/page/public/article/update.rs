use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::infrastructure::limits::use_limits;
use crate::page::author_gate::{denied_view, use_author_gate};
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::{validate_summary, validate_tags, validate_title};

#[component]
pub fn UpdateArticle() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();
    let limits = use_limits();

    let title = RwSignal::new(String::new());
    let summary = RwSignal::new(String::new());
    let tags = RwSignal::new(String::new());
    let loaded = RwSignal::new(false);

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
                    summary.set(view.summary);
                    tags.set(
                        view.tags
                            .iter()
                            .map(|tag| tag.name.clone())
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                    loaded.set(true);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    });

    let submit_notifications = notifications.clone();
    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        let Some(id) = params.get().get("article_id") else {
            return;
        };
        let limits = limits.get();
        let title_value = match validate_title(&title.get(), limits.max_title_chars) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&submit_notifications, &error);
                return;
            }
        };
        let summary_value = match validate_summary(&summary.get(), limits.max_summary_chars) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&submit_notifications, &error);
                return;
            }
        };
        if let Err(error) = validate_tags(&tags.get(), limits.max_tags_per_article as usize) {
            notify_error(&submit_notifications, &error);
            return;
        }
        let tags_value = tags.get();
        let notifications = submit_notifications.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            match crate::request::article::update_article(
                &id,
                &title_value,
                &summary_value,
                &tags_value,
            )
            .await
            {
                Ok(_) => {
                    notify_success(&notifications, "article updated");
                    navigate(
                        &format!("/public/article/{id}"),
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
        let submit = submit.clone();
        if denied.get() && checked.get() {
            return denied_view();
        }
        if !checked.get() || !loaded.get() {
            return view! { <p>loading...</p> }.into_any();
        }
        view! {
            <form on:submit=submit>
                <p>title</p>
                <input type="text" prop:value=title on:input=move |event| title.set(event_target_value(&event))/>
                <p>summary</p>
                <textarea prop:value=summary on:input=move |event| summary.set(event_target_value(&event))></textarea>
                <p>tags</p>
                <input type="text" prop:value=tags on:input=move |event| tags.set(event_target_value(&event))/>
                <button type="submit">update</button>
            </form>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
