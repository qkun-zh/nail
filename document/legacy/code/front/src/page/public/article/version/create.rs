use crate::limits::use_limits;
use crate::page::auth_gate::{denied_view, use_author_gate, who_are_you};
use crate::page::notify::{notify_error, notify_success, use_notify};
use common::text::validate_ascii_text;
use gloo_storage::{LocalStorage, Storage};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate, use_params_map};
use wasm_bindgen::JsCast;

#[component]
pub fn CreateVersion() -> impl IntoView {
    let notification = use_notify();
    let params = use_params_map();
    let location = use_location();
    let navigate = use_navigate();
    let article_id = params.get_untracked().get("article_id").unwrap_or_default();

    let url_params = location.query.get_untracked();
    let new_version = RwSignal::new(url_params.get("version").unwrap_or_default());
    let new_note = RwSignal::new(url_params.get("note").unwrap_or_default());
    let pathname = location.pathname.get_untracked();

    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            for (key, value) in [("version", new_version.get()), ("note", new_note.get())] {
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
        let _ = (new_version.get(), new_note.get());
        if prev.is_none() {
            return;
        }
        sync_url();
    });

    let uploading = RwSignal::new(false);

    let gate_id = article_id.clone();
    let (denied, checked) = use_author_gate(move || Some((Some(gate_id.clone()), None, None)));

    let limits = use_limits();

    let article_id_for_submit = article_id.clone();
    let on_create_version = move |ev: SubmitEvent| {
        ev.prevent_default();
        if uploading.get() {
            return;
        }
        let token = LocalStorage::get::<String>(crate::req::SESSION_TOKEN_KEY).unwrap_or_default();
        if token.is_empty() {
            notify_error(&notification, "authenticate first");
            return;
        }
        let version_val = new_version.get().trim().to_string();
        if version_val.is_empty() {
            notify_error(&notification, "enter a version");
            return;
        }
        let note_val =
            match validate_ascii_text(&new_note.get(), limits.get().max_version_note_chars, true) {
                Ok(normalized) => normalized,
                Err(e) => {
                    notify_error(&notification, &format!("invalid note: {e}"));
                    return;
                }
            };
        let file_input = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("version_pdf"))
            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok());
        let Some(file_input) = file_input else {
            notify_error(&notification, "no file input found");
            return;
        };
        let Some(file) = file_input.files().and_then(|files| files.get(0)) else {
            notify_error(&notification, "select a PDF file");
            return;
        };
        let file_type = file.type_();
        let looks_like_pdf = file_type == "application/pdf"
            || file_type.is_empty()
            || file_type == "application/octet-stream"
            || file.name().to_lowercase().ends_with(".pdf");
        if !looks_like_pdf {
            notify_error(&notification, "only PDF files are allowed");
            return;
        }
        if file.size() as u64 > limits.get().max_pdf_size_bytes {
            notify_error(&notification, "file too large");
            return;
        }

        uploading.set(true);
        let article_id = article_id_for_submit.clone();
        spawn_local(async move {
            match crate::req::create_article_version(
                &token,
                &article_id,
                &version_val,
                &note_val,
                file,
            )
            .await
            {
                Ok(data) => {
                    let version_id = data
                        .get("version_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !version_id.is_empty() {
                        notify_success(&notification, "version created");
                    } else {
                        notify_error(&notification, "version created but missing version id");
                    }
                }
                Err(e) => {
                    notify_error(&notification, &format!("create version failed: {e}"));
                }
            }
            uploading.set(false);
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
            } else if !checked.get() {
                view! { <p>loading...</p> }.into_any()
            } else {
                view! {
                    <form on:submit={on_create_version.clone()}>
                        <div>
                            <label><input type="text" required placeholder="version" bind:value=new_version/></label>
                        </div>
                        <div>
                            <label><textarea rows="4" cols=60 required placeholder="note: what changed in this version" bind:value=new_note></textarea></label>
                        </div>
                        <div>
                            <label><input id="version_pdf" type="file" accept="application/pdf"/></label>
                        </div>
                        <button type="submit" disabled=move || uploading.get()>
                            {move || if uploading.get() { "uploading..." } else { "create version" }}
                        </button>
                    </form>
                }.into_any()
            }
        }}
    }
}
