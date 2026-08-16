pub mod pagination;
pub mod render;
pub mod url;

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::hooks::{query_signal, use_navigate, use_params_map, use_query_map};
use nail_common::request::DeleteMode;
use nail_common::response::comment::{CommentListPage, CommentView};

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, notify_success, use_notifications};
use crate::page::session_gate::who_are_you;
use crate::page::validation::validate_comment_content;

use pagination::{COMMENTS_PER_PAGE, LevelPagination};
use render::{STYLE, comment_form, comment_rows, context_card};
use url::{CommentLevel, comment_id_from_level, comment_level_from_path};

#[component]
pub fn CommentSection() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();
    let query = use_query_map();
    let notifications = use_notifications();
    let limits = use_limits();

    let article_id = move || params.get().get("article_id").unwrap_or_default();
    let version_id_param = move || params.get().get("version_id").unwrap_or_default();
    let comment_path = move || params.get().get("comment_path").unwrap_or_default();
    let mode = move || comment_level_from_path(&comment_path());
    let (page_signal, _set_page) = query_signal::<u64>("page");
    let page = Memo::new(move |_| page_signal.get().unwrap_or(1).max(1));
    let base = move || {
        format!(
            "/public/article/{}/version/{}",
            article_id(),
            version_id_param()
        )
    };

    let roots = RwSignal::new(None::<CommentListPage>);
    let target = RwSignal::new(None::<CommentView>);
    let children = RwSignal::new(None::<CommentListPage>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(true);
    let posting = RwSignal::new(false);
    let delete_mode = RwSignal::new(DeleteMode::Transfer);
    let body = RwSignal::new(query.get_untracked().get("body").unwrap_or_default());
    let reply_body = RwSignal::new(query.get_untracked().get("reply").unwrap_or_default());

    let sync_url = {
        let navigate = navigate.clone();
        move || {
            let base_path = base();
            let page_value = page.get();
            let mut pairs: Vec<(String, String)> = Vec::new();
            let pathname = match mode() {
                CommentLevel::VersionComments => {
                    let value = body.get();
                    if !value.trim().is_empty() {
                        pairs.push(("body".to_string(), value));
                    }
                    format!("{base_path}/comment")
                }
                CommentLevel::Comment(comment_id) => {
                    let value = reply_body.get();
                    if !value.trim().is_empty() {
                        pairs.push(("reply".to_string(), value));
                    }
                    format!("{base_path}/comment/{comment_id}")
                }
                CommentLevel::DeleteComment(_) | CommentLevel::Invalid => return,
            };
            pairs.push(("page".to_string(), page_value.to_string()));
            let refs: Vec<(&str, &str)> = pairs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect();
            navigate(
                &crate::page::draft::draft_url(&pathname, &refs),
                leptos_router::NavigateOptions {
                    replace: true,
                    resolve: false,
                    ..Default::default()
                },
            );
        }
    };

    Effect::new(move |previous: Option<()>| {
        let _ = (body.get(), reply_body.get(), comment_path(), page.get());
        if previous.is_none() {
            return;
        }
        sync_url();
    });

    let load = StoredValue::new({
        let notifications = notifications.clone();
        move |version_id: String| {
            let current_mode = comment_level_from_path(&comment_path());
            let page_value = page.get();
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
                        let loading = loading;
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
                CommentLevel::DeleteComment(_) => {
                    loading.set(false);
                    error.set(None);
                }
                CommentLevel::Invalid => {
                    error.set(Some("comment not found".to_string()));
                    loading.set(false);
                }
            }
        }
    });

    Effect::new(move |_| {
        let _ = page.get();
        let _ = comment_path();
        let version_id = version_id_param();
        let load = load.get_value();
        load(version_id);
    });

    let comment_notifications = notifications.clone();
    let reply_notifications = notifications.clone();
    let delete_notifications = notifications.clone();
    let on_submit_comment = move |event: SubmitEvent| {
        event.prevent_default();
        if posting.get() {
            return;
        }
        let content =
            match validate_comment_content(&body.get(), limits.get().max_comment_body_chars) {
                Ok(value) => value,
                Err(error) => {
                    notify_error(&comment_notifications, &error);
                    return;
                }
            };
        let version_id = version_id_param();
        posting.set(true);
        let notifications = comment_notifications.clone();
        let load = load.get_value();
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
    };

    let on_submit_reply = move |event: SubmitEvent| {
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
                notify_error(&reply_notifications, &error);
                return;
            }
        };
        let comment_id = comment_id.to_string();
        posting.set(true);
        let notifications = reply_notifications.clone();
        let load = load.get_value();
        let version_id = version_id_param();
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
    };

    let on_submit_delete = {
        let load = load.get_value();
        move |delete_mode: DeleteMode| {
            if posting.get() {
                return;
            }
            let binding = mode();
            let Some(comment_id) = comment_id_from_level(&binding) else {
                return;
            };
            let comment_id = comment_id.to_string();
            let version_id = version_id_param();
            let base_path = base();
            posting.set(true);
            let notifications = delete_notifications.clone();
            let load = load.clone();
            let navigate = navigate.clone();
            leptos::task::spawn_local(async move {
                let result =
                    crate::request::comment::delete_comment(&comment_id, delete_mode).await;
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
    };

    view! {
        <section class="cmt-section">
            <style>{STYLE}</style>
            {move || {
                if let Some(message) = error.get() {
                    return view! { <p class="cmt-empty">{message}</p> }.into_any();
                }
                if loading.get() {
                    return view! { <p class="cmt-loading">loading comments...</p> }.into_any();
                }
                let base_path = base();
                let current_page = page.get();
                let authenticated = !crate::request::session::read_session_token()
                    .unwrap_or_default()
                    .is_empty();
                match mode() {
                    CommentLevel::VersionComments => {
                        let Some(list) = roots.get() else {
                            return ().into_any();
                        };
                        let has_next = list.has_next;
                        let rows = comment_rows(
                            &list.comments,
                            &base_path,
                            (current_page - 1) * COMMENTS_PER_PAGE,
                        );
                        let form = if authenticated {
                            comment_form(
                                body,
                                posting,
                                limits.get().max_comment_body_chars,
                                "comment",
                                "comment",
                                on_submit_comment.clone(),
                            )
                            .into_any()
                        } else {
                            who_are_you()
                        };
                        let list_view = if rows.is_empty() {
                            view! { <p class="cmt-empty">no comments yet</p> }.into_any()
                        } else {
                            view! { <div class="cmt-list">{rows}</div> }.into_any()
                        };
                        view! {
                            <div>
                                {form}
                                {list_view}
                                <LevelPagination
                                    current=current_page
                                    has_next=has_next
                                    base_href=format!("{base_path}/comment")
                                />
                            </div>
                        }
                        .into_any()
                    }
                    CommentLevel::Comment(comment_id) => {
                        let Some(comment) = target.get() else {
                            return view! { <p class="cmt-empty">comment not found</p> }.into_any();
                        };
                        let delete_url = format!("{base_path}/comment/{comment_id}/delete");
                        let form = if authenticated {
                            comment_form(
                                reply_body,
                                posting,
                                limits.get().max_comment_body_chars,
                                "reply",
                                "reply",
                                on_submit_reply.clone(),
                            )
                            .into_any()
                        } else {
                            who_are_you()
                        };
                        let child_list = children.get();
                        let rows = child_list
                            .as_ref()
                            .map(|list| {
                                comment_rows(
                                    &list.comments,
                                    &base_path,
                                    (current_page - 1) * COMMENTS_PER_PAGE,
                                )
                            })
                            .unwrap_or_default();
                        let has_next = child_list
                            .as_ref()
                            .is_some_and(|list| list.has_next);
                        let children_view = if rows.is_empty() {
                            view! { <p class="cmt-empty">no replies yet</p> }.into_any()
                        } else {
                            view! { <div class="cmt-list">{rows}</div> }.into_any()
                        };
                        view! {
                            <div>
                                {context_card(&comment, Some(delete_url))}
                                {form}
                                {children_view}
                                <LevelPagination
                                    current=current_page
                                    has_next=has_next
                                    base_href=format!("{base_path}/comment/{comment_id}")
                                />
                            </div>
                        }
                        .into_any()
                    }
                    CommentLevel::DeleteComment(_) => {
                        if !authenticated {
                            return who_are_you();
                        }
                        let submit_delete = on_submit_delete.clone();
                        let delete_mode = delete_mode.clone();
                        let is_transfer = move || delete_mode.get() == DeleteMode::Transfer;
                        let is_soft = move || delete_mode.get() == DeleteMode::Soft;
                        let is_hard = move || delete_mode.get() == DeleteMode::Hard;
                        view! {
                            <div>
                                <p class="cmt-empty">confirm delete comment</p>
                                <div>
                                    <label>
                                        <input type="radio" name="comment_delete_mode" prop:checked=is_transfer on:change=move |_| delete_mode.set(DeleteMode::Transfer)/>
                                        "transfer"
                                    </label>
                                </div>
                                <div>
                                    <label>
                                        <input type="radio" name="comment_delete_mode" prop:checked=is_soft on:change=move |_| delete_mode.set(DeleteMode::Soft)/>
                                        "soft"
                                    </label>
                                </div>
                                <div>
                                    <label>
                                        <input type="radio" name="comment_delete_mode" prop:checked=is_hard on:change=move |_| delete_mode.set(DeleteMode::Hard)/>
                                        "hard"
                                    </label>
                                </div>
                                <div>
                                    <button
                                        class="cmt-btn cmt-btn-danger"
                                        disabled=move || posting.get()
                                        on:click=move |_| submit_delete(delete_mode.get())
                                    >
                                        {move || if posting.get() { "deleting..." } else { "delete" }}
                                    </button>
                                </div>
                            </div>
                        }
                        .into_any()
                    }
                    CommentLevel::Invalid => {
                        view! { <p class="cmt-empty">comment not found</p> }.into_any()
                    }
                }
            }}
        </section>
    }
}
