use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::infrastructure::limits::use_limits;
use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::{
    validate_note, validate_pdf_selection, validate_summary, validate_tags, validate_title,
};

#[component]
pub fn CreateArticle() -> impl IntoView {
    let navigate = use_navigate();
    let query = use_query_map();
    let notifications = use_notifications();
    let limits = use_limits();

    let title = RwSignal::new(query.get_untracked().get("title").unwrap_or_default());
    let summary = RwSignal::new(query.get_untracked().get("summary").unwrap_or_default());
    let tags = RwSignal::new(query.get_untracked().get("tags").unwrap_or_default());
    let version = RwSignal::new(query.get_untracked().get("version").unwrap_or_default());
    let note = RwSignal::new(query.get_untracked().get("note").unwrap_or_default());
    let file_ref = NodeRef::<Input>::new();

    persist_draft(
        navigate.clone(),
        "/public/article/create".to_string(),
        move || {
            vec![
                ("title", title.get()),
                ("summary", summary.get()),
                ("tags", tags.get()),
                ("version", version.get()),
                ("note", note.get()),
            ]
        },
    );

    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        let limits = limits.get();
        let title_value = match validate_title(&title.get(), limits.max_title_chars) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };
        let summary_value = match validate_summary(&summary.get(), limits.max_summary_chars) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };
        if let Err(error) = validate_tags(&tags.get(), limits.max_tags_per_article as usize) {
            notify_error(&notifications, &error);
            return;
        }
        let note_value = match validate_note(&note.get(), limits.max_version_note_chars) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };
        let Some(file) = file_ref
            .get()
            .and_then(|input| input.files())
            .and_then(|files| files.get(0))
        else {
            notify_error(&notifications, "select a PDF file");
            return;
        };
        if let Err(error) = validate_pdf_selection(
            &file.type_(),
            &file.name(),
            file.size() as u64,
            limits.max_pdf_size_bytes,
        ) {
            notify_error(&notifications, &error);
            return;
        }

        let form = match build_form(&title_value, &summary_value, &tags.get(), &version.get(), &note_value, &file) {
            Ok(form) => form,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };

        let notifications = notifications.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            match crate::request::article::create_article(form).await {
                Ok(view) => {
                    notify_success(&notifications, "article created");
                    navigate(
                        &format!("/public/article/{}", view.article_id),
                        NavigateOptions {
                            resolve: false,
                            ..Default::default()
                        },
                    );
                }
                Err(error) => notify_error(&notifications, &error.to_string()),
            }
        });
    };

    view! {
            <div>
            <form on:submit=submit>
                <p>title</p>
                <input type="text" prop:value=title on:input=move |event| title.set(event_target_value(&event))/>
                <p>summary</p>
                <textarea prop:value=summary on:input=move |event| summary.set(event_target_value(&event))></textarea>
                <p>tags (space separated, each starting with #)</p>
                <input type="text" prop:value=tags on:input=move |event| tags.set(event_target_value(&event))/>
                <p>version (semver)</p>
                <input type="text" prop:value=version on:input=move |event| version.set(event_target_value(&event))/>
                <p>note</p>
                <textarea prop:value=note on:input=move |event| note.set(event_target_value(&event))></textarea>
                <p>pdf</p>
                <input type="file" accept="application/pdf" node_ref=file_ref/>
                <button type="submit">publish</button>
            </form>
            </div>
    }
}

fn build_form(
    title: &str,
    summary: &str,
    tags: &str,
    version: &str,
    note: &str,
    file: &web_sys::File,
) -> Result<web_sys::FormData, String> {
    let form = web_sys::FormData::new().map_err(|error| format!("failed to create FormData: {error:?}"))?;
    for (field, value) in [
        ("title", title),
        ("summary", summary),
        ("tags", tags),
        ("version", version),
        ("note", note),
    ] {
        form.append_with_str(field, value)
            .map_err(|error| format!("failed to append {field}: {error:?}"))?;
    }
    form.append_with_blob("file", file)
        .map_err(|error| format!("failed to append file: {error:?}"))?;
    Ok(form)
}
