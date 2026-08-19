use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::time_format::format_timestamp;
use crate::page::validation::validate_uuid;

#[component]
pub fn UserArticle() -> impl IntoView {
    let params = use_params_map();
    let notifications = use_notifications();
    let uid = move || params.get().get("uid").unwrap_or_default();
    let articles = RwSignal::new(None::<Vec<nail_common::response::article::ArticleListItem>>);

    Effect::new(move |_| {
        let id = uid();
        let notifications = notifications.clone();
        if let Err(error) = validate_uuid(&id) {
            notify_error(&notifications, error);
            return;
        }
        leptos::task::spawn_local(async move {
            match crate::request::user::read_user(&id).await {
                Ok(view) => articles.set(view.articles),
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    });

    let render = move || {
        let Some(list) = articles.get() else {
            return view! { <p>"loading..."</p> }.into_any();
        };
        if list.is_empty() {
            return view! { <p>"no articles"</p> }.into_any();
        }
        let rows = list
            .into_iter()
            .map(|a| {
                let href = format!("/article/{}", a.id);
                let created_at = format_timestamp(a.created_at);
                view! {
                    <div>
                        <A href=href>{a.title}</A>
                        <span>" "</span>
                        <span>{created_at}</span>
                    </div>
                }
            })
            .collect_view();
        view! { <div>{rows}</div> }.into_any()
    };

    view! { <div>{render}</div> }
}
