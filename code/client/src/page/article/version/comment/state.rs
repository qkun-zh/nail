use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;

use common::request::DeleteMode;
use common::response::RuntimeLimits;

use crate::page::notify::{Notifications, notify_error, notify_success};
use crate::page::validation::validate_comment_content;

use super::url::{CommentLevel, comment_id_from_level};

/// Everything the comment submit builders share.
#[derive(Clone)]
pub struct CommentSubmit {
    pub notifications: Notifications,
    pub posting: RwSignal<bool>,
    pub limits: RwSignal<RuntimeLimits>,
}

fn refresh_options() -> NavigateOptions {
    NavigateOptions {
        replace: true,
        resolve: false,
        ..Default::default()
    }
}

/// Run one comment mutation with the shared guard/toast/refresh protocol.
pub fn run_comment_action<T, A, S>(submit: &CommentSubmit, action: A, on_success: S)
where
    T: 'static,
    A: Future<Output = Result<T, crate::request::error::RequestError>> + 'static,
    S: FnOnce(&Notifications) + 'static,
{
    let posting = submit.posting;
    let notifications = submit.notifications.clone();
    leptos::task::spawn_local(async move {
        let result = action.await;
        posting.set(false);
        match result {
            Ok(_) => on_success(&notifications),
            Err(error) => notify_error(&notifications, error.to_string()),
        }
    });
}

/// Guard + content validation shared by the text-adding comment forms.
fn checked_content(event: &SubmitEvent, submit: &CommentSubmit, raw: &str) -> Option<String> {
    event.prevent_default();
    if submit.posting.get() {
        return None;
    }
    match validate_comment_content(raw, submit.limits.get().max_comment_body_chars) {
        Ok(content) => Some(content),
        Err(error) => {
            notify_error(&submit.notifications, &error);
            None
        }
    }
}

fn current_comment_id(mode: &dyn Fn() -> CommentLevel) -> Option<String> {
    comment_id_from_level(&mode()).map(str::to_string)
}

pub fn build_submit_comment(
    submit: CommentSubmit,
    body: RwSignal<String>,
    version_id: impl Fn() -> String + Clone + 'static,
    load: impl Fn() + Clone + 'static,
) -> impl Fn(SubmitEvent) + Clone + 'static {
    move |event: SubmitEvent| {
        let Some(content) = checked_content(&event, &submit, &body.get()) else {
            return;
        };
        let load = load.clone();
        let id = version_id();
        run_comment_action(
            &submit,
            async move { crate::request::comment::create_comment(&id, &content).await },
            move |notifications| {
                body.set(String::new());
                notify_success(notifications, "comment created");
                load();
            },
        );
    }
}

pub fn build_submit_reply(
    submit: CommentSubmit,
    reply_body: RwSignal<String>,
    mode: impl Fn() -> CommentLevel + Clone + 'static,
    load: impl Fn() + Clone + 'static,
) -> impl Fn(SubmitEvent) + Clone + 'static {
    move |event: SubmitEvent| {
        let Some(comment_id) = current_comment_id(&mode) else {
            return;
        };
        let Some(content) = checked_content(&event, &submit, &reply_body.get()) else {
            return;
        };
        let load = load.clone();
        run_comment_action(
            &submit,
            async move { crate::request::comment::create_reply(&comment_id, &content).await },
            move |notifications| {
                reply_body.set(String::new());
                notify_success(notifications, "reply created");
                load();
            },
        );
    }
}

pub fn build_submit_update(
    submit: CommentSubmit,
    update_body: RwSignal<String>,
    mode: impl Fn() -> CommentLevel + Clone + 'static,
    base: impl Fn() -> String + Clone + 'static,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    load: impl Fn() + Clone + 'static,
) -> impl Fn(SubmitEvent) + Clone + 'static {
    move |event: SubmitEvent| {
        let Some(comment_id) = current_comment_id(&mode) else {
            return;
        };
        let Some(content) = checked_content(&event, &submit, &update_body.get()) else {
            return;
        };
        let target = format!("{}/comment/{comment_id}", base());
        let navigate = navigate.clone();
        let load = load.clone();
        run_comment_action(
            &submit,
            async move { crate::request::comment::update_comment(&comment_id, &content).await },
            move |notifications| {
                notify_success(notifications, "comment updated");
                navigate(&target, refresh_options());
                load();
            },
        );
    }
}

pub fn build_submit_undelete(
    submit: CommentSubmit,
    mode: impl Fn() -> CommentLevel + Clone + 'static,
    base: impl Fn() -> String + Clone + 'static,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    load: impl Fn() + Clone + 'static,
) -> impl Fn() + Clone + 'static {
    move || {
        let Some(comment_id) = current_comment_id(&mode) else {
            return;
        };
        let target = format!("{}/comment/{comment_id}", base());
        let navigate = navigate.clone();
        let load = load.clone();
        run_comment_action(
            &submit,
            async move { crate::request::comment::undelete_soft_comment(&comment_id).await },
            move |notifications| {
                notify_success(notifications, "comment restored");
                navigate(&target, refresh_options());
                load();
            },
        );
    }
}

pub fn build_submit_delete(
    submit: CommentSubmit,
    mode: impl Fn() -> CommentLevel + Clone + 'static,
    base: impl Fn() -> String + Clone + 'static,
    navigate: impl Fn(&str, NavigateOptions) + Clone + 'static,
    load: impl Fn() + Clone + 'static,
) -> impl Fn(DeleteMode) + Clone + 'static {
    move |delete_mode: DeleteMode| {
        let Some(comment_id) = current_comment_id(&mode) else {
            return;
        };
        let target = format!("{}/comment", base());
        let navigate = navigate.clone();
        let load = load.clone();
        run_comment_action(
            &submit,
            async move { crate::request::comment::delete_comment(&comment_id, delete_mode).await },
            move |notifications| {
                let message = match delete_mode {
                    DeleteMode::Transfer => "comment transferred to recycler",
                    DeleteMode::Hard => "comment deleted",
                    DeleteMode::Soft => "comment soft-deleted",
                };
                notify_success(notifications, message);
                navigate(&target, refresh_options());
                load();
            },
        );
    }
}
