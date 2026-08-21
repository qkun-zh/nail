use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::infrastructure::limits::use_limits;
use crate::page::article::tag_picker::TagPicker;
use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::{
    validate_note, validate_pdf_selection, validate_summary, validate_title,
};

#[component]
pub fn CreateArticle() -> impl IntoView {
    let navigate = use_navigate();
    let query = use_query_map();
    let notifications = use_notifications();
    let limits = use_limits();

    let title = RwSignal::new(query.get_untracked().get("title").unwrap_or_default());
    let summary = RwSignal::new(query.get_untracked().get("summary").unwrap_or_default());
    let tags_from_query = query.get_untracked().get("tags").unwrap_or_default();
    let selected_tags = RwSignal::new(
        tags_from_query
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    );
    let version = RwSignal::new(query.get_untracked().get("version").unwrap_or_default());
    let note = RwSignal::new(query.get_untracked().get("note").unwrap_or_default());
    let file_ref = NodeRef::<Input>::new();
    let working = RwSignal::new(false);

    persist_draft(navigate.clone(), "/article/create".to_string(), move || {
        vec![
            ("title", title.get()),
            ("summary", summary.get()),
            ("tags", selected_tags.get().join(" ")),
            ("version", version.get()),
            ("note", note.get()),
        ]
    });

    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
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
        let tags_value = selected_tags.get().join(" ");
        if tags_value.is_empty() {
            notify_error(&notifications, "at least one tag is required");
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
            crate::infrastructure::js::js_number_to_u64(file.size()),
            limits.max_pdf_size_bytes,
        ) {
            notify_error(&notifications, &error);
            return;
        }

        let form = match build_form(
            &title_value,
            &summary_value,
            &tags_value,
            &version.get(),
            &note_value,
            &file,
        ) {
            Ok(form) => form,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };

        working.set(true);
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::article::create_article(form).await;
            working.set(false);
            match result {
                Ok(view) => {
                    let message = if view.version_id.is_empty() {
                        format!("article created: {}", view.article_id)
                    } else {
                        format!(
                            "article created: {} (version {})",
                            view.article_id, view.version_id
                        )
                    };
                    notify_success(&notifications, message);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    };

    view! {
        <form on:submit=submit>
            <div><label><input type="text" placeholder="title" prop:value=title on:input=move |event| title.set(event_target_value(&event)) /></label></div>
            <div><label><textarea rows="6" cols="60" placeholder="summary" prop:value=summary on:input=move |event| summary.set(event_target_value(&event))></textarea></label></div>
            <div>
                <label>"Tags"</label>
                <TagPicker selected=selected_tags />
            </div>
            <div><label><input type="text" placeholder="version" prop:value=version on:input=move |event| version.set(event_target_value(&event)) /></label></div>
            <div><label><textarea rows="4" cols="60" placeholder="note: what changed in this version" prop:value=note on:input=move |event| note.set(event_target_value(&event))></textarea></label></div>
            <div><label><input type="file" accept="application/pdf" node_ref=file_ref /></label></div>
            <button type="submit" disabled=move || working.get()>
                {move || if working.get() { "creating..." } else { "create article" }}
            </button>
        </form>
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
    let form = web_sys::FormData::new()
        .map_err(|error| format!("failed to create FormData: {error:?}"))?;
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
    form.append_with_str("file_name", file.name().as_str())
        .map_err(|error| format!("failed to append file_name: {error:?}"))?;
    form.append_with_str("content_type", file.type_().as_str())
        .map_err(|error| format!("failed to append content_type: {error:?}"))?;
    form.append_with_blob_and_filename("file", file, &file.name())
        .map_err(|error| format!("failed to append file: {error:?}"))?;
    Ok(form)
}
