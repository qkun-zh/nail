use crate::page::auth_gate::{denied_view, use_author_gate, use_component_alive, who_are_you};
use crate::page::notify::{notify_error, notify_success, use_notify};
use gloo_storage::{LocalStorage, Storage};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

#[component]
pub fn DeleteArticle() -> impl IntoView {
    let notification = use_notify();
    let params = use_params_map();
    let alive = use_component_alive();

    let title = RwSignal::new(String::new());
    let loaded = RwSignal::new(false);
    let missing_id = RwSignal::new(false);
    let submitting = RwSignal::new(false);
    let request_seq = StoredValue::new(0u64);

    let (denied, checked) = use_author_gate(move || {
        let article_id = params.get().get("article_id").unwrap_or_default();
        if article_id.trim().is_empty() {
            None
        } else {
            Some((Some(article_id), None, None))
        }
    });

    Effect::new({
        let alive = alive.clone();
        move |_| {
            let article_id = params.get().get("article_id").unwrap_or_default();
            let my_seq = request_seq.get_value() + 1;
            request_seq.set_value(my_seq);
            title.set(String::new());
            missing_id.set(false);
            loaded.set(false);
            if article_id.trim().is_empty() {
                missing_id.set(true);
                loaded.set(true);
                notify_error(&notification, "missing article id");
                return;
            }
            if !checked.get() || denied.get() {
                return;
            }
            spawn_local({
                let alive = alive.clone();
                async move {
                    match crate::req::read_article_detail(&article_id).await {
                        Ok(data) => {
                            if !alive.get_value() {
                                return;
                            }
                            if request_seq.get_value() != my_seq {
                                return;
                            }
                            if let Some(t) = data.get("title").and_then(|v| v.as_str()) {
                                title.set(t.to_string());
                            }
                            loaded.set(true);
                        }
                        Err(e) => {
                            if !alive.get_value() {
                                return;
                            }
                            if request_seq.get_value() != my_seq {
                                return;
                            }
                            notify_error(&notification, &format!("load failed: {e}"));
                            loaded.set(true);
                        }
                    }
                }
            });
        }
    });

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
        let article_id = params.get().get("article_id").unwrap_or_default();
        submitting.set(true);
        spawn_local({
            let alive = alive.clone();
            async move {
                match crate::req::delete_article(&token, &article_id).await {
                    Ok(_) => {
                        if !alive.get_value() {
                            return;
                        }
                        notify_success(&notification, "article deleted");
                    }
                    Err(e) => {
                        if !alive.get_value() {
                            return;
                        }
                        notify_error(&notification, &format!("delete failed: {e}"));
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
            } else if missing_id.get() {
                view! { <p>missing article id</p> }.into_any()
            } else {
                view! {
                    <form on:submit={on_submit.clone()}>
                        {if title.get().is_empty() {
                            None
                        } else {
                            let title_value = title.get();
                            Some(view! { <div>{format!("title){title_value}")}</div> })
                        }}
                        <button type="submit" disabled=move || submitting.get()>
                            {move || if submitting.get() { "deleting..." } else { "delete" }}
                        </button>
                    </form>
                }.into_any()
            }
        }}
    }
}
