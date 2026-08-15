use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use crate::infrastructure::limits::use_limits;
use crate::page::author_gate::{denied_view, use_author_gate};
use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::{validate_summary, validate_tags, validate_title};

#[component]
pub fn UpdateArticle() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let notifications = use_notifications();
    let limits = use_limits();
    let query = use_query_map();

    let title = RwSignal::new(query.get_untracked().get("title").unwrap_or_default());
    let summary = RwSignal::new(query.get_untracked().get("summary").unwrap_or_default());
    let tags = RwSignal::new(query.get_untracked().get("tags").unwrap_or_default());
    let loaded = RwSignal::new(false);
    let working = RwSignal::new(false);

    let article_id = move || params.get().get("article_id");
    let (denied, checked) = use_author_gate(article_id);

    persist_draft(
        navigate.clone(),
        format!(
            "/public/article/{}/update",
            params.get_untracked().get("article_id").unwrap_or_default()
        ),
        move || {
            vec![
                ("title", title.get()),
                ("summary", summary.get()),
                ("tags", tags.get()),
            ]
        },
    );

    let effect_notifications = notifications.clone();
    Effect::new(move |_| {
        let Some(id) = params.get().get("article_id") else {
            return;
        };
        let notifications = effect_notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::article::read_article(&id).await {
                Ok(view) => {
                    if title.get_untracked().is_empty() {
                        title.set(view.title);
                    }
                    if summary.get_untracked().is_empty() {
                        summary.set(view.summary);
                    }
                    if tags.get_untracked().is_empty() {
                        tags.set(
                            view.tags
                                .iter()
                                .map(|tag| tag.name.clone())
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                    }
                    loaded.set(true);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    });

    let submit_notifications = notifications.clone();
    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
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
        if let Err(error) = validate_tags(
            &tags.get(),
            usize::try_from(limits.max_tags_per_article).unwrap_or(usize::MAX),
        ) {
            notify_error(&submit_notifications, &error);
            return;
        }
        let tags_value = tags.get();
        working.set(true);
        let notifications = submit_notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::article::update_article(
                &id,
                &title_value,
                &summary_value,
                &tags_value,
            )
            .await;
            working.set(false);
            match result {
                Ok(_) => notify_success(&notifications, "article updated"),
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
                <div><label><input type="text" placeholder="title" prop:value=title on:input=move |event| title.set(event_target_value(&event)) /></label></div>
                <div><label><textarea rows="6" cols="60" placeholder="summary" prop:value=summary on:input=move |event| summary.set(event_target_value(&event))></textarea></label></div>
                <div><label><textarea rows="6" cols="60" placeholder="tag (space separated)" prop:value=tags on:input=move |event| tags.set(event_target_value(&event))></textarea></label></div>
                <button type="submit" disabled=move || working.get()>
                    {move || if working.get() { "saving..." } else { "save" }}
                </button>
            </form>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}
