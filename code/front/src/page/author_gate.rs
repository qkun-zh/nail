use leptos::prelude::*;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::validation::validate_uuid;

pub fn denied_view() -> AnyView {
    view! { <p>you are denied!</p> }.into_any()
}

pub fn use_author_gate(
    article_id: impl Fn() -> Option<String> + Copy + 'static,
) -> (RwSignal<bool>, RwSignal<bool>) {
    let notification = use_notifications();
    let denied = RwSignal::new(false);
    let checked = RwSignal::new(false);
    let sequence = StoredValue::new(0u64);

    Effect::new(move |_| {
        let my_sequence = sequence.get_value() + 1;
        sequence.set_value(my_sequence);

        let Some(id) = article_id() else {
            denied.set(false);
            checked.set(true);
            return;
        };
        if id.trim().is_empty() {
            denied.set(false);
            checked.set(true);
            return;
        }
        if let Err(message) = validate_uuid(&id) {
            notify_error(&notification, message);
            checked.set(true);
            return;
        }
        checked.set(false);
        let notification = notification.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::article::read_article(&id).await;
            if sequence.get_value() != my_sequence {
                return;
            }
            match result {
                Ok(article) => {
                    let current_user = crate::page::session_gate::authenticated_user_id();
                    denied.set(Some(article.author_id) != current_user);
                }
                Err(error) => {
                    notify_error(&notification, format!("author check failed: {error}"));
                }
            }
            checked.set(true);
        });
    });

    (denied, checked)
}
