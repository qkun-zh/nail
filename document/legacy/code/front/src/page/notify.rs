use gloo_timers::callback::Interval;
use leptos::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

const HISTORY_CAP: usize = 100;

thread_local! {
    static TICK_INTERVAL: RefCell<Option<Interval>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug)]
pub enum NotificationType {
    Info,
    Success,
    Error,
}

impl NotificationType {
    fn css_class(&self) -> &'static str {
        match self {
            NotificationType::Error => "ntf-error",
            NotificationType::Success => "ntf-success",
            NotificationType::Info => "ntf-info",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Notification {
    pub id: u64,
    pub message: String,
    pub reference: Option<String>,
    pub kind: NotificationType,
    pub created_ms: f64,
    pub duration_ms: u64,
}

impl Notification {
    fn remaining_secs(&self, now_ms: f64) -> u64 {
        let elapsed = (now_ms - self.created_ms).max(0.0) as u64;
        let remaining = self.duration_ms.saturating_sub(elapsed);
        remaining.div_ceil(1000)
    }

    fn full_content(&self) -> String {
        match &self.reference {
            Some(reference) => format!("{} [{}]", self.message, reference),
            None => self.message.clone(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct NotifyHandle {
    pub notifications: RwSignal<Vec<Notification>>,
    pub history: RwSignal<Vec<Notification>>,
    pub tick: RwSignal<u64>,
}

pub fn use_notify() -> NotifyHandle {
    use_context::<NotifyHandle>().expect("NotifyHandle: not provided at app root")
}

pub fn provide_notify() -> NotifyHandle {
    let handle = NotifyHandle {
        notifications: RwSignal::new(Vec::new()),
        history: RwSignal::new(Vec::new()),
        tick: RwSignal::new(0),
    };
    provide_context(handle);
    handle
}

fn start_tick(handle: &NotifyHandle) {
    TICK_INTERVAL.with(|cell| {
        if cell.borrow().is_none() {
            let handle = *handle;
            *cell.borrow_mut() = Some(Interval::new(1000, move || {
                handle.tick.update(|tick| *tick = tick.wrapping_add(1));
            }));
        }
    });
}

fn stop_tick_if_idle(handle: &NotifyHandle) {
    let idle = handle.notifications.read_untracked().is_empty();
    if idle {
        TICK_INTERVAL.with(|cell| {
            *cell.borrow_mut() = None;
        });
    }
}

pub fn notify(
    handle: &NotifyHandle,
    message: &str,
    reference: Option<String>,
    kind: NotificationType,
    duration_ms: u64,
) {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let notification = Notification {
        id,
        message: message.to_string(),
        reference,
        kind,
        created_ms: js_sys::Date::now(),
        duration_ms,
    };
    handle
        .notifications
        .update(|list| list.push(notification.clone()));
    handle.history.update(|list| {
        list.push(notification.clone());
        let overflow = list.len().saturating_sub(HISTORY_CAP);
        if overflow > 0 {
            list.drain(..overflow);
        }
    });
    start_tick(handle);

    let handle = *handle;
    set_timeout(
        move || {
            handle
                .notifications
                .update(|list| list.retain(|n| n.id != id));
            stop_tick_if_idle(&handle);
        },
        std::time::Duration::from_millis(duration_ms),
    );
}

pub fn notify_error(handle: &NotifyHandle, message: &str) {
    notify(handle, message, None, NotificationType::Error, 5000);
}

pub fn notify_success(handle: &NotifyHandle, message: &str) {
    notify(handle, message, None, NotificationType::Success, 3000);
}

pub fn dismiss(handle: &NotifyHandle, id: u64) {
    handle
        .notifications
        .update(|list| list.retain(|n| n.id != id));
    stop_tick_if_idle(handle);
}

const STYLE: &str = r#"
.ntf-container {
    position: fixed;
    top: 16px;
    right: 16px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 420px;
    pointer-events: none;
}
.ntf-toast {
    pointer-events: auto;
    padding: 10px 14px;
    border-radius: 8px;
    box-shadow: 0 4px 14px rgba(0,0,0,0.12);
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 14px;
    line-height: 1.5;
    word-break: break-word;
    animation: ntf-in 0.25s ease-out;
    border: 1px solid;
}
.ntf-error {
    background: #fef2f2;
    color: #991b1b;
    border-color: #fecaca;
}
.ntf-success {
    background: #f0fdf4;
    color: #166534;
    border-color: #bbf7d0;
}
.ntf-info {
    background: #eff6ff;
    color: #1e40af;
    border-color: #bfdbfe;
}
.ntf-seq {
    flex-shrink: 0;
    font-weight: 600;
    min-width: 1.5em;
    text-align: right;
}
.ntf-body {
    flex: 1;
    min-width: 0;
}
.ntf-countdown {
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    opacity: 0.7;
}
.ntf-close {
    flex-shrink: 0;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 0 2px;
    opacity: 0.5;
    color: inherit;
}
.ntf-close:hover {
    opacity: 1;
}
.ntf-header {
    pointer-events: auto;
    display: flex;
    justify-content: flex-end;
}
.ntf-history-toggle {
    background: none;
    border: 1px solid rgba(0,0,0,0.15);
    border-radius: 6px;
    padding: 2px 8px;
    font-size: 12px;
    cursor: pointer;
    opacity: 0.7;
    color: inherit;
}
.ntf-history-toggle:hover {
    opacity: 1;
}
.ntf-history {
    pointer-events: auto;
    background: #ffffff;
    color: #1f2937;
    border: 1px solid rgba(0,0,0,0.12);
    border-radius: 8px;
    box-shadow: 0 4px 14px rgba(0,0,0,0.12);
    max-height: 320px;
    overflow-y: auto;
    font-size: 13px;
    line-height: 1.5;
}
.ntf-history-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid rgba(0,0,0,0.06);
    word-break: break-word;
}
.ntf-history-row:last-child {
    border-bottom: none;
}
.ntf-history-row.ntf-error {
    background: #fef2f2;
    color: #991b1b;
}
.ntf-history-row.ntf-success {
    background: #f0fdf4;
    color: #166534;
}
.ntf-history-row.ntf-info {
    background: #eff6ff;
    color: #1e40af;
}
@keyframes ntf-in {
    from { transform: translateX(100%); opacity: 0; }
    to   { transform: translateX(0);    opacity: 1; }
}
"#;

#[component]
pub fn NotifyDisplay() -> impl IntoView {
    let handle = use_notify();
    let show_history = RwSignal::new(false);

    view! {
        <style>{STYLE}</style>
        <div class="ntf-container">
            <div class="ntf-header">
                <button
                    class="ntf-history-toggle"
                    on:click=move |_| show_history.update(|shown| *shown = !*shown)
                >
                    {move || if show_history.get() { "hide history" } else { "history" }}
                </button>
            </div>
            {move || {
                handle.notifications.get().into_iter().map(|notification| {
                    let id = notification.id;
                    let css = notification.kind.css_class().to_string();
                    let content = notification.full_content();
                    let countdown = {
                        let notification_for_countdown = notification.clone();
                        move || {
                            let _ = handle.tick.get();
                            let remaining = notification_for_countdown
                                .remaining_secs(js_sys::Date::now());
                            if remaining == 0 {
                                String::new()
                            } else {
                                format!("{}s", remaining)
                            }
                        }
                    };
                    view! {
                        <div class={format!("ntf-toast {}", css)}>
                            <span class="ntf-seq">{id}</span>
                            <span class="ntf-body">{content}</span>
                            <span class="ntf-countdown">{countdown}</span>
                            <button class="ntf-close" on:click=move |_| dismiss(&handle, id)>x</button>
                        </div>
                    }
                }).collect::<Vec<_>>()
            }}
            {move || {
                if show_history.get() {
                    let entries = handle.history.get();
                    view! {
                        <div class="ntf-history">
                            {entries.iter().rev().map(|notification| {
                                let css = notification.kind.css_class().to_string();
                                let content = notification.full_content();
                                view! {
                                    <div class={format!("ntf-history-row {}", css)}>
                                        <span class="ntf-seq">{notification.id}</span>
                                        <span class="ntf-body">{content}</span>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}
        </div>
    }
}
