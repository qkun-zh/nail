use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;

use nail_common::request::DeleteMode;
use nail_common::response::ListPage;
use nail_common::response::RuntimeLimits;
use nail_common::response::comment::CommentView;

use crate::page::notify::{Notifications, notify_error, notify_success};
use crate::page::validation::validate_comment_content;

use super::pagination::COMMENTS_PER_PAGE;
use super::url::{CommentLevel, comment_id_from_level, comment_level_from_path};

#[derive(Clone, Copy)]
pub struct CommentSignals {
    pub loading: RwSignal<bool>,
    pub roots: RwSignal<Option<ListPage<CommentView>>>,
    pub target: RwSignal<Option<CommentView>>,
    pub children: RwSignal<Option<ListPage<CommentView>>>,
    pub error: RwSignal<Option<String>>,
}

pub fn build_load(
    notifications: Notifications,
    signals: CommentSignals,
    comment_path: impl Fn() -> String + Clone + 'static,
    page: impl Fn() -> u64 + Clone + 'static,
) -> impl Fn(String) + Clone + 'static {
    let CommentSignals {
        loading,
        roots,
        target,
        children,
        error,
    } = signals;
    move |version_id: String| {
        let current_mode = comment_level_from_path(&comment_path());
        let page_value = page();
        let notifications = notifications.clone();
        match current_mode {
            CommentLevel::VersionComments => {
                loading.set(true);
                leptos::task::spawn_local(async move {
                    match crate::request::comment::read_comments(
                        &version_id,
                        page_value,
                        COMMENTS_PER_PAGE,
                    )
                    .await
                    {
                        Ok(view) => {
                            roots.set(Some(view));
                            error.set(None);
                        }
                        Err(request_error) => {
                            notify_error(&notifications, request_error.to_string());
                            error.set(Some(request_error.to_string()));
                        }
                    }
                    loading.set(false);
                });
            }
            CommentLevel::Comment(comment_id) => {
                loading.set(true);
                let comment_id = comment_id.clone();
                let notifications = notifications.clone();
                let pending = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(2));
                let done_loading = {
                    let pending = pending.clone();
                    move || {
                        if pending.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 1 {
                            loading.set(false);
                        }
                    }
                };
                leptos::task::spawn_local({
                    let comment_id = comment_id.clone();
                    let notifications = notifications.clone();
                    let done_loading = done_loading.clone();
                    async move {
                        match crate::request::comment::read_comment(&comment_id).await {
                            Ok(view) => target.set(Some(view)),
                            Err(request_error) => {
                                notify_error(&notifications, request_error.to_string());
                                error.set(Some(request_error.to_string()));
                            }
                        }
                        done_loading();
                    }
                });
                leptos::task::spawn_local({
                    let comment_id = comment_id.clone();
                    let notifications = notifications.clone();
                    let done_loading = done_loading.clone();
                    async move {
                        match crate::request::comment::read_comment_children(
                            &comment_id,
                            page_value,
                            COMMENTS_PER_PAGE,
                        )
                        .await
                        {
                            Ok(view) => children.set(Some(view)),
                            Err(request_error) => {
                                notify_error(&notifications, request_error.to_string());
                                error.set(Some(request_error.to_string()));
                            }
                        }
                        done_loading();
                    }
                });
            }
            CommentLevel::DeleteComment(_)
            | CommentLevel::UpdateComment(_)
            | CommentLevel::UndeleteComment(_) => {
                loading.set(false);
                error.set(None);
            }
            CommentLevel::Invalid => {
                error.set(Some("comment not found".to_string()));
                loading.set(false);
            }
        }
    }
}

pub fn build_submit_comment(
    notifications: Notifications,
    posting: RwSignal<bool>,
    body: RwSignal<String>,
    limits: RwSignal<RuntimeLimits>,
    version_id: impl Fn() -> String + Clone + 'static,
    load: impl Fn(String) + Clone + 'static,
) -> impl Fn(SubmitEvent) + Clone + 'static {
    move |event: SubmitEvent| {
        event.prevent_default();
        if posting.get() {
            return;
        }
        let content =
            match validate_comment_content(&body.get(), limits.get().max_comment_body_chars) {
                Ok(value) => value,
                Err(error) => {
                    notify_error(&notifications, &error);
                    return;
                }
            };
        let version_id = version_id();
        posting.set(true);
        let notifications = notifications.clone();
        let load = load.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::comment::create_comment(&version_id, &content).await;
            posting.set(false);
            match result {
                Ok(_) => {
                    notify_success(&notifications, "comment created");
                    load(version_id);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    }
}

pub fn build_submit_reply(
    notifications: Notifications,
    posting: RwSignal<bool>,
    reply_body: RwSignal<String>,
    limits: RwSignal<RuntimeLimits>,
    mode: impl Fn() -> CommentLevel + Clone + 'static,
    version_id: impl Fn() -> String + Clone + 'static,
    load: impl Fn(String) + Clone + 'static,
) -> impl Fn(SubmitEvent) + Clone + 'static {
    move |event: SubmitEvent| {
        event.prevent_default();
        if posting.get() {
            return;
        }
        let binding = mode();
        let Some(comment_id) = comment_id_from_level(&binding) else {
            return;
        };
        let content = match validate_comment_content(
            &reply_body.get(),
            limits.get().max_comment_body_chars,
        ) {
            Ok(value) => value,
            Err(error) => {
                notify_error(&notifications, &error);
                return;
            }
        };
        let comment_id = comment_id.to_string();
        posting.set(true);
        let notifications = notifications.clone();
        let load = load.clone();
        let version_id = version_id();
        leptos::task::spawn_local(async move {
            let result = crate::request::comment::create_reply(&comment_id, &content).await;
            posting.set(false);
            match result {
                Ok(_) => {
                    notify_success(&notifications, "reply created");
                    load(version_id);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_submit_update(
    notifications: Notifications,
    posting: RwSignal<bool>,
    update_body: RwSignal<String>,
    limits: RwSignal<RuntimeLimits>,
    mode: impl Fn() -> CommentLevel + Clone + 'static,
    version_id: impl Fn() -> String + Clone + 'static,
    base: impl Fn() -> String + Clone + 'static,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    load: impl Fn(String) + Clone + 'static,
) -> impl Fn(SubmitEvent) + Clone + 'static {
    move |event: SubmitEvent| {
        event.prevent_default();
        if posting.get() {
            return;
        }
        let binding = mode();
        let Some(comment_id) = comment_id_from_level(&binding) else {
            return;
        };
        let content =
            match validate_comment_content(&update_body.get(), limits.get().max_comment_body_chars)
            {
                Ok(value) => value,
                Err(error) => {
                    notify_error(&notifications, &error);
                    return;
                }
            };
        let comment_id = comment_id.to_string();
        let version_id = version_id();
        let base_path = base();
        posting.set(true);
        let notifications = notifications.clone();
        let load = load.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::comment::update_comment(&comment_id, &content).await;
            posting.set(false);
            match result {
                Ok(_) => {
                    notify_success(&notifications, "comment updated");
                    navigate(
                        &crate::page::draft::draft_url(
                            &format!("{base_path}/comment/{comment_id}"),
                            &[],
                        ),
                        leptos_router::NavigateOptions {
                            replace: true,
                            resolve: false,
                            ..Default::default()
                        },
                    );
                    load(version_id);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    }
}

pub fn build_submit_undelete(
    notifications: Notifications,
    posting: RwSignal<bool>,
    mode: impl Fn() -> CommentLevel + Clone + 'static,
    version_id: impl Fn() -> String + Clone + 'static,
    base: impl Fn() -> String + Clone + 'static,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    load: impl Fn(String) + Clone + 'static,
) -> impl Fn() + Clone + 'static {
    move || {
        if posting.get() {
            return;
        }
        let binding = mode();
        let Some(comment_id) = comment_id_from_level(&binding) else {
            return;
        };
        let comment_id = comment_id.to_string();
        let version_id = version_id();
        let base_path = base();
        posting.set(true);
        let notifications = notifications.clone();
        let load = load.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::comment::undelete_soft_comment(&comment_id).await;
            posting.set(false);
            match result {
                Ok(_) => {
                    notify_success(&notifications, "comment restored");
                    navigate(
                        &crate::page::draft::draft_url(
                            &format!("{base_path}/comment/{comment_id}"),
                            &[],
                        ),
                        leptos_router::NavigateOptions {
                            replace: true,
                            resolve: false,
                            ..Default::default()
                        },
                    );
                    load(version_id);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    }
}

pub fn build_submit_delete(
    notifications: Notifications,
    posting: RwSignal<bool>,
    mode: impl Fn() -> CommentLevel + Clone + 'static,
    version_id: impl Fn() -> String + Clone + 'static,
    base: impl Fn() -> String + Clone + 'static,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    load: impl Fn(String) + Clone + 'static,
) -> impl Fn(DeleteMode) + Clone + 'static {
    move |delete_mode: DeleteMode| {
        if posting.get() {
            return;
        }
        let binding = mode();
        let Some(comment_id) = comment_id_from_level(&binding) else {
            return;
        };
        let comment_id = comment_id.to_string();
        let version_id = version_id();
        let base_path = base();
        posting.set(true);
        let notifications = notifications.clone();
        let load = load.clone();
        let navigate = navigate.clone();
        leptos::task::spawn_local(async move {
            let result = crate::request::comment::delete_comment(&comment_id, delete_mode).await;
            posting.set(false);
            match result {
                Ok(_) => {
                    let message = match delete_mode {
                        DeleteMode::Transfer => "comment transferred to recycler",
                        DeleteMode::Hard => "comment deleted",
                        DeleteMode::Soft => "comment soft-deleted",
                    };
                    notify_success(&notifications, message);
                    navigate(
                        &crate::page::draft::draft_url(&format!("{base_path}/comment"), &[]),
                        leptos_router::NavigateOptions {
                            replace: true,
                            resolve: false,
                            ..Default::default()
                        },
                    );
                    load(version_id);
                }
                Err(error) => notify_error(&notifications, error.to_string()),
            }
        });
    }
}
