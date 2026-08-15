use crate::limits::use_limits;
use crate::page::auth_gate::who_are_you;
use crate::page::notify::{notify_error, notify_success, use_notify};
use common::tag::parse_hashtag_tags;
use common::text::validate_ascii_text;
use gloo_storage::{LocalStorage, Storage};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate};
use wasm_bindgen::JsCast;

const TEXTAREA_COLS: u32 = 60;

struct ResetSubmitGuard(RwSignal<bool>);

impl Drop for ResetSubmitGuard {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

#[component]
pub fn CreateArticle() -> impl IntoView {
    let notification = use_notify();
    let location = use_location();
    let navigate = use_navigate();

    let url_params = location.query.get_untracked();
    let title = RwSignal::new(url_params.get("title").unwrap_or_default());
    let summary = RwSignal::new(url_params.get("summary").unwrap_or_default());
    let tags_raw = RwSignal::new(url_params.get("tags").unwrap_or_default());
    let version = RwSignal::new(url_params.get("version").unwrap_or_default());
    let note = RwSignal::new(url_params.get("note").unwrap_or_default());
    let pathname = location.pathname.get_untracked();

    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            for (key, value) in [
                ("title", title.get()),
                ("summary", summary.get()),
                ("tags", tags_raw.get()),
                ("version", version.get()),
                ("note", note.get()),
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
        let _ = (
            title.get(),
            summary.get(),
            tags_raw.get(),
            version.get(),
            note.get(),
        );
        if prev.is_none() {
            return;
        }
        sync_url();
    });

    let submitting = RwSignal::new(false);

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
        let title_val = title.get();
        if let Err(e) = validate_ascii_text(&title_val, limits.get().max_title_chars, false) {
            notify_error(&notification, &format!("invalid title: {e}"));
            return;
        }
        let summary_val = summary.get();
        if let Err(e) = validate_ascii_text(&summary_val, limits.get().max_summary_chars, true) {
            notify_error(&notification, &format!("invalid summary: {e}"));
            return;
        }
        let tags_raw_val = tags_raw.get();
        if let Err(e) = parse_hashtag_tags(&tags_raw_val, limits.get().max_tags_per_article) {
            notify_error(&notification, &format!("invalid tags: {e}"));
            return;
        }
        let version_val = version.get();
        if version_val.trim().is_empty() {
            notify_error(&notification, "enter a version");
            return;
        }
        let note_val = note.get();
        if let Err(e) = validate_ascii_text(&note_val, limits.get().max_version_note_chars, true) {
            notify_error(&notification, &format!("invalid note: {e}"));
            return;
        }
        let file_input = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("article_pdf"))
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

        submitting.set(true);
        spawn_local(async move {
            let _submitting_guard = ResetSubmitGuard(submitting);
            match crate::req::create_article(
                &token,
                &title_val,
                &summary_val,
                &tags_raw_val,
                &version_val,
                &note_val,
                file,
            )
            .await
            {
                Ok(data) => {
                    let article_id = data
                        .get("article_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let version_id = data
                        .get("version_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let msg = if version_id.is_empty() {
                        format!("article created: {article_id}")
                    } else {
                        format!("article created: {article_id} (version {version_id})")
                    };
                    notify_success(&notification, &msg);
                }
                Err(e) => {
                    notify_error(&notification, &format!("create failed: {e}"));
                }
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
            } else {
                view! {
                <form on:submit=on_submit>
                    <div>
                        <label><input type="text" placeholder="title" required bind:value=title/></label>
                    </div>
                    <div>
                        <label><textarea rows="6" cols=TEXTAREA_COLS placeholder="summary" required bind:value=summary></textarea></label>
                    </div>
                    <div>
                        <label><textarea rows="6" cols=TEXTAREA_COLS placeholder="tag" bind:value=tags_raw></textarea></label>
                    </div>
                    <div>
                        <label><input type="text" required placeholder="version" bind:value=version/></label>
                    </div>
                    <div>
                        <label><textarea rows="4" cols=TEXTAREA_COLS required placeholder="note: what changed in this version" bind:value=note></textarea></label>
                    </div>
                    <div>
                        <label><input id="article_pdf" type="file" accept="application/pdf"/></label>
                    </div>
                    <button type="submit" disabled=move || submitting.get()>
                        {move || if submitting.get() { "creating..." } else { "create article" }}
                    </button>
                </form>
            }.into_any()
        }}
    }
    }
}
