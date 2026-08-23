use leptos::prelude::*;

use crate::page::fetch::{LoadError, require_id};
use crate::page::notify::{notify_error, use_notifications};

pub fn denied_view() -> AnyView {
    view! { <p>you are denied!</p> }.into_any()
}

pub fn use_author_gate(
    article_id: impl Fn() -> Option<String> + Copy + 'static,
) -> (RwSignal<bool>, RwSignal<bool>) {
    let denied = RwSignal::new(false);
    let checked = RwSignal::new(false);

    let verdict: LocalResource<Result<bool, LoadError>> = LocalResource::new(move || {
        let id = article_id();
        async move {
            match id.filter(|id| !id.trim().is_empty()) {
                None => Ok(false),
                Some(id) => {
                    let id = require_id(&id)?;
                    let article = crate::request::article::read_article(&id).await?;
                    let current_user = crate::page::session_gate::authenticated_user_id();
                    Ok(Some(article.author_id) != current_user)
                }
            }
        }
    });

    let notification = use_notifications();
    Effect::new(move |_| match verdict.get() {
        Some(Ok(is_denied)) => {
            denied.set(is_denied);
            checked.set(true);
        }
        Some(Err(message)) => {
            notify_error(&notification, format!("author check failed: {message}"));
            checked.set(true);
        }
        None => checked.set(false),
    });

    (denied, checked)
}
