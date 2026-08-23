use std::future::Future;

use leptos::callback::UnsyncCallback;
use leptos::prelude::*;

use crate::page::fetch::LoadError;
use crate::page::notify::{notify_error, use_notifications};

/// Shared submit machinery for confirmation pages.
#[derive(Clone, Copy)]
pub struct ConfirmHandle {
    pub working: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
    pub submit: UnsyncCallback<()>,
}

/// Run `action` when the handle's submit fires. Failures are toasted and shown
/// inline; the button disables while the action is in flight.
pub fn use_confirm_action<F, Fut>(action: F) -> ConfirmHandle
where
    F: Fn() -> Fut + Clone + 'static,
    Fut: Future<Output = Result<(), LoadError>> + 'static,
{
    let working = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let notifications = use_notifications();

    let submit = UnsyncCallback::new(move |(): ()| {
        if working.get() {
            return;
        }
        working.set(true);
        error.set(None);
        let action = action.clone();
        let notifications = notifications.clone();
        leptos::task::spawn_local(async move {
            match action().await {
                Ok(()) => working.set(false),
                Err(failure) => {
                    let message = failure.to_string();
                    notify_error(&notifications, message.clone());
                    error.set(Some(message));
                    working.set(false);
                }
            }
        });
    });

    ConfirmHandle {
        working,
        error,
        submit,
    }
}

/// Standard confirmation button wired to a [`ConfirmHandle`].
#[component]
pub fn ConfirmButton(
    handle: ConfirmHandle,
    #[prop(into)] label: String,
    #[prop(into)] busy_label: String,
) -> impl IntoView {
    view! {
        {move || handle.error.get().map(|error| view! { <p class="error">{error}</p> })}
        <button on:click=move |_| handle.submit.run(()) disabled=handle.working>
            {move || if handle.working.get() { busy_label.clone() } else { label.clone() }}
        </button>
    }
}
