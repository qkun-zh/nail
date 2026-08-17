use std::sync::atomic::{AtomicU64, Ordering};

use leptos::prelude::*;

pub const TOAST_DURATION_MS: u32 = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub kind: NotificationType,
    pub message: String,
}

pub fn kind_class(kind: NotificationType) -> &'static str {
    match kind {
        NotificationType::Success => "success",
        NotificationType::Error => "error",
    }
}

static NEXT_TOAST_ID: AtomicU64 = AtomicU64::new(1);

fn next_toast_id() -> u64 {
    NEXT_TOAST_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone)]
pub struct Notifications {
    pub toasts: RwSignal<Vec<Toast>>,
}

impl Notifications {
    pub fn new() -> Self {
        Self {
            toasts: RwSignal::new(Vec::new()),
        }
    }

    pub fn push(&self, kind: NotificationType, message: impl Into<String>) {
        let toast = Toast {
            id: next_toast_id(),
            kind,
            message: message.into(),
        };
        self.toasts.update(|list| list.push(toast.clone()));

        let id = toast.id;
        let toasts = self.toasts;
        gloo_timers::callback::Timeout::new(TOAST_DURATION_MS, move || {
            toasts.update(|list| list.retain(|toast| toast.id != id));
        })
        .forget();
    }
}

pub fn provide_notifications() -> Notifications {
    let notifications = Notifications::new();
    provide_context(notifications.clone());
    notifications
}

pub fn use_notifications() -> Notifications {
    use_context::<Notifications>().unwrap_or_else(Notifications::new)
}

pub fn notify_error(notifications: &Notifications, message: impl Into<String>) {
    notifications.push(NotificationType::Error, message);
}

pub fn notify_success(notifications: &Notifications, message: impl Into<String>) {
    notifications.push(NotificationType::Success, message);
}

#[component]
pub fn ToastContainer() -> impl IntoView {
    let notifications = use_notifications();
    let source = notifications.clone();
    let toasts = move || {
        source
            .toasts
            .get()
            .into_iter()
            .map(|toast| {
                let class = format!("toast toast--{}", kind_class(toast.kind));
                view! {
                    <div class=class role="status">
                        <span class="toast-dot"></span>
                        <span class="toast-message">{toast.message}</span>
                    </div>
                }
            })
            .collect_view()
    };
    view! { <div class="toast-container">{toasts}</div> }
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/notify/tests.rs"]
mod tests;
