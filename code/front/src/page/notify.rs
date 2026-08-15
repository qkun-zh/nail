use std::sync::atomic::{AtomicU64, Ordering};

use leptos::prelude::*;

const HISTORY_CAP: usize = 100;
const TICK_MS: u32 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    #[allow(dead_code)]
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub id: u64,
    pub kind: NotificationType,
    pub message: String,
    pub expires_at_ms: u64,
}

pub fn toast_duration_ms(kind: NotificationType) -> u64 {
    match kind {
        NotificationType::Error => 5_000,
        NotificationType::Success | NotificationType::Info => 3_000,
    }
}

pub fn remaining_seconds(expires_at_ms: u64, now_ms: u64) -> u64 {
    expires_at_ms.saturating_sub(now_ms).div_ceil(1_000)
}

pub fn capped_insert(history: &mut Vec<Toast>, toast: Toast, cap: usize) {
    history.push(toast);
    if history.len() > cap {
        history.drain(..history.len() - cap);
    }
}

pub fn kind_label(kind: NotificationType) -> &'static str {
    match kind {
        NotificationType::Info => "info",
        NotificationType::Success => "success",
        NotificationType::Error => "error",
    }
}

static NEXT_TOAST_ID: AtomicU64 = AtomicU64::new(1);

fn next_toast_id() -> u64 {
    NEXT_TOAST_ID.fetch_add(1, Ordering::Relaxed)
}

fn current_time_ms() -> u64 {
    u64::try_from(js_sys::Date::now()).unwrap_or(u64::MAX)
}

#[derive(Clone)]
pub struct Notifications {
    pub toasts: RwSignal<Vec<Toast>>,
    pub history: RwSignal<Vec<Toast>>,
    pub history_visible: RwSignal<bool>,
    pub now_ms: RwSignal<u64>,
}

impl Notifications {
    pub fn new() -> Self {
        Self {
            toasts: RwSignal::new(Vec::new()),
            history: RwSignal::new(Vec::new()),
            history_visible: RwSignal::new(false),
            now_ms: RwSignal::new(current_time_ms()),
        }
    }

    pub fn start_ticker(&self) {
        let now_signal = self.now_ms;
        gloo_timers::callback::Interval::new(TICK_MS, move || now_signal.set(current_time_ms()))
            .forget();
    }

    pub fn push(&self, kind: NotificationType, message: impl Into<String>) {
        let toast = Toast {
            id: next_toast_id(),
            kind,
            message: message.into(),
            expires_at_ms: current_time_ms() + toast_duration_ms(kind),
        };
        self.toasts.update(|list| list.push(toast.clone()));

        let mut history = self.history.get_untracked();
        capped_insert(&mut history, toast.clone(), HISTORY_CAP);
        self.history.set(history);

        let id = toast.id;
        let toasts = self.toasts;
        gloo_timers::callback::Timeout::new(
            u32::try_from(toast_duration_ms(kind)).unwrap_or(u32::MAX),
            move || {
                toasts.update(|list| list.retain(|toast| toast.id != id));
            },
        )
        .forget();
    }

    pub fn dismiss(&self, id: u64) {
        self.toasts
            .update(|list| list.retain(|toast| toast.id != id));
    }

    pub fn toggle_history(&self) {
        self.history_visible.update(|visible| *visible = !*visible);
    }
}

pub fn provide_notifications() -> Notifications {
    let notifications = Notifications::new();
    notifications.start_ticker();
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

#[allow(dead_code)]
pub fn notify_info(notifications: &Notifications, message: impl Into<String>) {
    notifications.push(NotificationType::Info, message);
}

#[component]
pub fn ToastContainer() -> impl IntoView {
    let notifications = use_notifications();

    let toasts_source = notifications.clone();
    let dismiss_buttons = move || {
        toasts_source
            .toasts
            .get()
            .into_iter()
            .map(|toast| {
                let notifications = toasts_source.clone();
                let id = toast.id;
                let message = toast.message.clone();
                let kind = toast.kind;
                let remaining = remaining_seconds(toast.expires_at_ms, notifications.now_ms.get());
                view! {
                    <div>
                        {kind_label(kind)}
                        {" · "}
                        {message}
                        {" · "}
                        {remaining}
                        <button on:click=move |_| notifications.dismiss(id)>dismiss</button>
                    </div>
                }
            })
            .collect_view()
    };

    let history_source = notifications.clone();
    let history_list = move || {
        if history_source.history_visible.get() {
            history_source
                .history
                .get()
                .into_iter()
                .map(|toast| {
                    view! {
                        <div>
                            {kind_label(toast.kind)}
                            {" · "}
                            {toast.message}
                        </div>
                    }
                })
                .collect_view()
                .into_any()
        } else {
            ().into_any()
        }
    };

    let toggle_source = notifications.clone();
    view! {
        <div>
            {dismiss_buttons}
            <button on:click=move |_| toggle_source.toggle_history()>history</button>
            {history_list}
        </div>
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/notify/tests.rs"]
mod tests;
