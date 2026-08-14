use leptos::prelude::*;
use leptos_router::components::A;
use nail_common::response::user::UserView;

use crate::page::notify::{notify_error, use_notifications};
use crate::page::session_gate::{SessionStatus, use_session_status};

#[component]
pub fn PrivateIndex() -> impl IntoView {
    let notifications = use_notifications();
    let status = use_session_status();
    let profile = RwSignal::new(None::<UserView>);

    Effect::new(move |_| {
        let SessionStatus::Authenticated(view) = status.get() else {
            return;
        };
        let Some(user_id) = view.id else {
            return;
        };
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match crate::request::user::read_self_user(&user_id, true, true).await {
                Ok(view) => profile.set(Some(view)),
                Err(error) => notify_error(&notifications, &error.to_string()),
            }
        });
    });

    view! {
            <div>
                <p>private area</p>
                {move || match profile.get() {
                    Some(view) => view! {
                        <div>
                            <p>name: {view.name.clone().unwrap_or_default()}</p>
                            <p>email hash: {view.email_hash.clone().unwrap_or_default()}</p>
                        </div>
                    }.into_any(),
                    None => view! { <p>loading profile...</p> }.into_any(),
                }}
                <A href="/private/name">name</A>
                <A href="/private/email">email</A>
                <A href="/private/logout">logout</A>
                <A href="/private/deregister">deregister</A>
            </div>
    }
}
