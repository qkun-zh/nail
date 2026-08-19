use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use crate::infrastructure::limits::use_limits;
use crate::page::author_gate::{denied_view, use_author_gate};
use crate::page::draft::persist_draft;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::validation::{validate_note, validate_pdf_selection, validate_uuid};

#[component]
pub fn CreateVersion() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let query = use_query_map();
    let notifications = use_notifications();
    let limits = use_limits();

    let version = RwSignal::new(query.get_untracked().get("version").unwrap_or_default());
    let note = RwSignal::new(query.get_untracked().get("note").unwrap_or_default());
    let file_ref = NodeRef::<Input>::new();
    let working = RwSignal::new(false);

    let article_id = move || params.get().get("article_id");
    let (denied, checked) = use_author_gate(article_id);

    let pathname = format!(
        "/article/{}/version/create",
        params.get_untracked().get("article_id").unwrap_or_default()
    );
    persist_draft(navigate.clone(), pathname, move || {
        vec![("version", version.get()), ("note", note.get())]
    });

    let submit = move |event: SubmitEvent| {
        event.prevent_default();
        if working.get() {
            return;
        }
        let Some(article_id) = params.get().get("article_id") else {
            return;
        };
        if let Err(message) = validate_uuid(&article_id) {
            notify_error(&notifications, message);
            return;
        }
        let version_value = version.get();
        if version_value.trim().is_empty() {
            notify_error(&notifications, "version is required");
            return;
        }
        let limits = limits.get();
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
        let form = match build_form(&version_value, &note_value, &file) {
            Ok(form) => form,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };
        working.set(true);
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::version::create_version(&article_id, form).await;
            working.set(false);
            match result {
                Ok(_) => notify_success(&notifications, "version created"),
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    };

    let render = move || {
        let submit = submit.clone();
        if denied.get() && checked.get() {
            return denied_view();
        }
        if !checked.get() {
            return view! { <p>loading...</p> }.into_any();
        }
        view! {
            <form on:submit=submit>
                <div><label><input type="text" placeholder="version" prop:value=version on:input=move |event| version.set(event_target_value(&event)) /></label></div>
                <div><label><textarea rows="4" cols="60" placeholder="note: what changed in this version" prop:value=note on:input=move |event| note.set(event_target_value(&event))></textarea></label></div>
                <div><label><input type="file" accept="application/pdf" node_ref=file_ref /></label></div>
                <button type="submit" disabled=move || working.get()>
                    {move || if working.get() { "uploading..." } else { "create version" }}
                </button>
            </form>
        }
        .into_any()
    };

    view! { <div>{render}</div> }
}

fn build_form(
    version: &str,
    note: &str,
    file: &web_sys::File,
) -> Result<web_sys::FormData, String> {
    let form = web_sys::FormData::new()
        .map_err(|error| format!("failed to create FormData: {error:?}"))?;
    for (field, value) in [("version", version), ("note", note)] {
        form.append_with_str(field, value)
            .map_err(|error| format!("failed to append {field}: {error:?}"))?;
    }
    form.append_with_blob("file", file)
        .map_err(|error| format!("failed to append file: {error:?}"))?;
    Ok(form)
}
