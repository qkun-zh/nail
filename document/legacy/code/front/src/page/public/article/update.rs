use crate::limits::use_limits;
use crate::page::auth_gate::{denied_view, use_author_gate, use_component_alive, who_are_you};
use crate::page::notify::{notify_error, notify_success, use_notify};
use common::tag::parse_hashtag_tags;
use common::text::validate_ascii_text;
use gloo_storage::{LocalStorage, Storage};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate, use_params_map};

#[component]
pub fn UpdateArticle() -> impl IntoView {
    let notification = use_notify();
    let params = use_params_map();
    let location = use_location();
    let navigate = use_navigate();
    let alive = use_component_alive();

    let url_params = location.query.get_untracked();
    let title = RwSignal::new(url_params.get("title").unwrap_or_default());
    let summary = RwSignal::new(url_params.get("summary").unwrap_or_default());
    let tags_raw = RwSignal::new(url_params.get("tags").unwrap_or_default());
    let loaded = RwSignal::new(true);
    let submitting = RwSignal::new(false);
    let pathname = location.pathname.get_untracked();

    let prefill_draft = (
        url_params.get("title").unwrap_or_default(),
        url_params.get("summary").unwrap_or_default(),
        url_params.get("tags").unwrap_or_default(),
    );
    let prefill_seq = StoredValue::new(0u64);
    let alive_prefill = alive.clone();
    Effect::new(move |_| {
        let article_id = params.get().get("article_id").unwrap_or_default();
        let my_seq = prefill_seq.get_value() + 1;
prefill_seq.set_value(my_seq);
        if article_id.trim().is_empty() {
            return;
        }
        let (old_title, old_summary, old_tags) = prefill_draft.clone();
        let alive = alive_prefill.clone();
        spawn_local(
            async move {
                let Ok(data) = crate::req::read_article_detail(&article_id).await else {
                    return;
                };
                if !alive.get_value() || prefill_seq.get_value() != my_seq {
                    return;
                }
                if old_title.is_empty() && title.get_untracked().is_empty() {
                    if let Some(v) = data.get("title").and_then(|v| v.as_str()) {
                        title.set(v.to_string());
                    }
                }
                if old_summary.is_empty() && summary.get_untracked().is_empty() {
                    if let Some(v) = data.get("summary").and_then(|v| v.as_str()) {
                        summary.set(v.to_string());
                    }
                }
                if old_tags.is_empty() && tags_raw.get_untracked().is_empty() {
                    let tags_raw_val = data
                        .get("tags")
                        .and_then(|v| v.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n");
                    tags_raw.set(tags_raw_val);
                }
            },
        );
    });

    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            for (key, value) in [
                ("title", title.get()),
                ("summary", summary.get()),
                ("tags", tags_raw.get()),
            ] {
                if !value.is_empty() {
                    pairs.push(format!("{}={}", key, crate::req::url_encode(&value)));
                }
            }
            let query_string = pairs.join("&");
            navigate(
                &format!("{pathname}?{query_string}"),
                leptos_router::NavigateOptions {
                    replace: true,
                    resolve: false,
                    ..Default::default()
                },
            );
        }
    };

    Effect::new(move |prev: Option<()>| {
        let _ = (title.get(), summary.get(), tags_raw.get());
        if prev.is_none() {
            return;
        }
        sync_url();
    });

    let (denied, checked) = use_author_gate(move || {
        let article_id = params.get().get("article_id").unwrap_or_default();
        if article_id.trim().is_empty() {
            None
        } else {
            Some((Some(article_id), None, None))
        }
    });

    let limits = use_limits();

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        if submitting.get() {
            return;
        }
        let token = LocalStorage::get::<String>(crate::req::SESSION_TOKEN_KEY).unwrap_or_default();
        if token.is_empty() {
            notify_error(&notification, "not logged in: authenticate first");
            return;
        }
        let title_val = match validate_ascii_text(&title.get(), limits.get().max_title_chars, false)
        {
            Ok(normalized) => normalized,
            Err(e) => {
                notify_error(&notification, &format!("invalid title: {e}"));
                return;
            }
        };
        let summary_val =
            match validate_ascii_text(&summary.get(), limits.get().max_summary_chars, true) {
                Ok(normalized) => normalized,
                Err(e) => {
                    notify_error(&notification, &format!("invalid summary: {e}"));
                    return;
                }
            };
        let tags_raw_val = tags_raw.get();
        if let Err(e) = parse_hashtag_tags(&tags_raw_val, limits.get().max_tags_per_article) {
            notify_error(&notification, &format!("invalid tags: {e}"));
            return;
        }
        let article_id = params.get().get("article_id").unwrap_or_default();
        submitting.set(true);
        spawn_local({
            let alive = alive.clone();
            async move {
                match crate::req::update_article(
                    &token,
                    &article_id,
                    &title_val,
                    &summary_val,
                    &tags_raw_val,
                )
                .await
                {
                    Ok(_) => {
                        if !alive.get_value() {
                            return;
                        }
                        notify_success(&notification, "article updated");
                    }
                    Err(e) => {
                        if !alive.get_value() {
                            return;
                        }
                        notify_error(&notification, &format!("update failed: {e}"));
                    }
                }
                submitting.set(false);
            }
        });
    };

    view! {
        {move || {
            let has_session = !LocalStorage::get::<String>(crate::req::SESSION_TOKEN_KEY)
                .unwrap_or_default()
                .is_empty();
            if !has_session {
                who_are_you()
            } else if denied.get() && checked.get() {
                denied_view()
            } else if !checked.get() || !loaded.get() {
                view! { <p>loading...</p> }.into_any()
            } else {
                view! {
                    <form on:submit={on_submit.clone()}>
                        <div>
                            <label><input type="text" placeholder="title" required bind:value=title/></label>
                        </div>
                        <div>
                            <label><textarea rows="6" cols=60 placeholder="summary" required bind:value=summary></textarea></label>
                        </div>
                        <div>
                            <label><textarea rows="6" cols=60 placeholder="tag (#a #b)" bind:value=tags_raw></textarea></label>
                        </div>
                        <button type="submit" disabled=move || submitting.get()>
                            {move || if submitting.get() { "saving..." } else { "save" }}
                        </button>
                    </form>
                }.into_any()
            }
        }}
    }
}
