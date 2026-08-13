
use crate::limits::use_limits;
use crate::page::auth_gate::{denied_view, use_author_gate, who_are_you};
use crate::page::notify::{notify_error, notify_success, use_notify};
use gloo_storage::{LocalStorage, Storage};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_location, use_navigate, use_params_map};

pub mod fetch;
pub mod pagination;
pub mod render;
pub mod style;
pub mod url;

pub use render::DownloadLink;

use common::text::validate_ascii_text;
use fetch::{build_comment_tree, fetch_version_comments};
use pagination::{level_paginator, paginate_level};
use render::{comment_form, comment_id, context_card, delete_comment_form, render_comment_rows};
use style::STYLE;
use url::{CommentLevel, comment_delete_id, comment_level_from_path, comment_level_id};

#[component]
pub fn CommentSection(version_id: String) -> impl IntoView {
    let notification = use_notify();
    let location = use_location();
    let navigate = use_navigate();

    let version_id_render = version_id.clone();

    let comments = RwSignal::new(Vec::<serde_json::Value>::new());
    let loaded = RwSignal::new(false);
    let comments_page = RwSignal::new(1u64);
    let comments_has_next = RwSignal::new(false);
    let posting = RwSignal::new(false);
    let session_token =
        RwSignal::new(LocalStorage::get::<String>("session_token").unwrap_or_default());
    let url_params = location.query.get_untracked();
    let comment_body = RwSignal::new(url_params.get("body").unwrap_or_default());
    let reply_body = RwSignal::new(url_params.get("reply").unwrap_or_default());
    let pathname = location.pathname.get_untracked();

    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            for (key, value) in [("body", comment_body.get()), ("reply", reply_body.get())] {
                if !value.is_empty() {
                    pairs.push(format!("{}={}", key, crate::req::url_encode(&value)));
                }
            }
            let query_string = pairs.join("&");
            navigate(
                &format!("{pathname}?{query_string}"),
                leptos_router::NavigateOptions {
                    replace: true,
                    resolve: false,
                    ..Default::default()
                },
            );
        }
    };

    Effect::new(move |prev: Option<()>| {
        let _ = (comment_body.get(), reply_body.get());
        if prev.is_none() {
            return;
        }
        sync_url();
    });

    let list_page = RwSignal::new(1u64);
    let params = use_params_map();

    let (delete_denied, delete_checked) = use_author_gate(move || {
        let params_map = params.get();
        let version_id = params_map.get("version_id").unwrap_or_default();
        comment_delete_id(&params_map.get("comment_path").unwrap_or_default())
            .map(|cid| (None, Some(version_id), Some(cid)))
    });

    Effect::new(move |_| {
        let _ = params.get();
        list_page.set(1);
    });

    let limits = use_limits();

    let reload = move |vid: String| {
        let comments = comments;
        let loaded = loaded;
        let notification = notification;
        let comments_page = comments_page;
        let comments_has_next = comments_has_next;
        let limits = limits;
        async move {
            let limit = limits.get().search_page_size.max(1) as u64;
            match fetch_version_comments(&vid, 1, limit).await {
                Ok(json) => {
                    comments_page.set(1);
                    if let Some(list) = json.get("comments").and_then(|v| v.as_array()) {
                        let mut list = list.clone();
                        list.sort_by_key(comment_id);
                        comments.set(list);
                    }
                    comments_has_next
                        .set(json.get("has_next").and_then(|v| v.as_bool()).unwrap_or(false));
                }
                Err(e) => {
                    notify_error(&notification, &format!("comments failed: {e}"));
                }
            }
            loaded.set(true);
        }
    };
    let load_more = move |vid: String| {
        let comments = comments;
        let notification = notification;
        let comments_page = comments_page;
        let comments_has_next = comments_has_next;
        let limits = limits;
        async move {
            let next_page = comments_page.get() + 1;
            let limit = limits.get().search_page_size.max(1) as u64;
            match fetch_version_comments(&vid, next_page, limit).await {
                Ok(json) => {
                    if let Some(list) = json.get("comments").and_then(|v| v.as_array()) {
                        let mut current = comments.get();
                        let mut fresh = list.clone();
                        fresh.sort_by_key(comment_id);
                        let mut seen: std::collections::HashSet<String> =
                            current.iter().map(comment_id).collect();
                        for item in fresh {
                            let id = comment_id(&item);
                            if seen.insert(id.clone()) {
                                current.push(item);
                            }
                        }
                        comments.set(current);
                    }
                    comments_page.set(next_page);
                    comments_has_next
                        .set(json.get("has_next").and_then(|v| v.as_bool()).unwrap_or(false));
                }
                Err(e) => {
                    notify_error(&notification, &format!("comments failed: {e}"));
                }
            }
        }
    };
    spawn_local(reload(version_id.clone()));

    let validate_body = move |body: &str| -> bool {
        match validate_ascii_text(body, limits.get().max_comment_body_chars, true) {
            Ok(_) => true,
            Err(e) => {
                notify_error(&notification, &e.to_string());
                false
            }
        }
    };

    let version_id_for_reply = version_id.clone();

    let on_submit_comment = move |ev: SubmitEvent| {
        ev.prevent_default();
        if posting.get() {
            return;
        }
        let token = session_token.get();
        if token.is_empty() {
            notify_error(&notification, "authenticate to comment");
            return;
        }
        let body = comment_body.get();
        if !validate_body(&body) {
            return;
        }
        let version_id_for_post = version_id.clone();
        posting.set(true);
        let reload = reload;
        spawn_local(async move {
            match crate::req::create_version_comment(&token, &version_id_for_post, &body).await {
                Ok(_) => {
                    notify_success(&notification, "comment posted");
                    reload(version_id_for_post.clone()).await;
                }
                Err(e) => {
                    notify_error(&notification, &format!("comment failed: {e}"));
                }
            }
            posting.set(false);
        });
    };

    let on_submit_reply = move |ev: SubmitEvent| {
        ev.prevent_default();
        if posting.get() {
            return;
        }
        let token = session_token.get();
        if token.is_empty() {
            notify_error(&notification, "authenticate to comment");
            return;
        }
        let comment_path = params.get().get("comment_path").unwrap_or_default();
        let Some(parent_id) = comment_level_id(&comment_path) else {
            return;
        };
        let body = reply_body.get();
        if !validate_body(&body) {
            return;
        }
        posting.set(true);
        let reload = reload;
        let version_id_for_reply_owned = version_id_for_reply.clone();
        spawn_local(async move {
            match crate::req::create_comment_reply(&token, &parent_id, &body).await {
                Ok(_) => {
                    notify_success(&notification, "reply posted");
                    reload(version_id_for_reply_owned).await;
                }
                Err(e) => {
                    notify_error(&notification, &format!("reply failed: {e}"));
                }
            }
            posting.set(false);
        });
    };

    let on_submit_delete_comment = move |ev: SubmitEvent| {
        ev.prevent_default();
        if posting.get() {
            return;
        }
        let token = session_token.get();
        if token.is_empty() {
            notify_error(&notification, "authenticate to delete");
            return;
        }
        let params_map = params.get();
        let comment_path = params_map.get("comment_path").unwrap_or_default();
        let Some(target_id) = comment_delete_id(&comment_path) else {
            return;
        };
        let version_id = params_map.get("version_id").unwrap_or_default();
        posting.set(true);
        spawn_local(async move {
            match crate::req::delete_comment(&token, &target_id).await {
                Ok(_) => {
                    notify_success(&notification, "comment deleted");
                    reload(version_id.clone()).await;
                }
                Err(e) => {
                    notify_error(&notification, &format!("delete failed: {e}"));
                }
            }
            posting.set(false);
        });
    };

    let version_id_render = version_id_render;

    view! {
        {move || {
            let params_map = params.get();
            let comment_path = params_map.get("comment_path").unwrap_or_default();
            match comment_level_from_path(&comment_path) {
                CommentLevel::DeleteComment(_) => ().into_any(),
                _ => view! { <style>{STYLE}</style> }.into_any(),
            }
        }}
        <section class="cmt-section">
            {move || {
                if !loaded.get() {
                    return view! { <p class="cmt-empty">loading comments...</p> }.into_any();
                }
                let comment_list = comments.get();
                let can_reply = !session_token.get().is_empty();
                let params_map = params.get();
                let comment_path = params_map.get("comment_path").unwrap_or_default();
                let article_id = params_map.get("article_id").unwrap_or_default();
                let url_version_id = params_map.get("version_id").unwrap_or_default();
                let base = format!("/public/article/{article_id}/version/{url_version_id}");

                let (roots, children) = build_comment_tree(&comment_list);
                let page = list_page.get();
                let mut parts: Vec<leptos::prelude::AnyView> = Vec::new();

                match comment_level_from_path(&comment_path) {
                    CommentLevel::VersionPage => {
                    }
                    CommentLevel::Invalid => {
                        parts.push(
                            view! { <p class="cmt-empty">comment not found</p> }.into_any(),
                        );
                    }
                    CommentLevel::VersionComments => {
                        if can_reply {
                            parts.push(comment_form(comment_body, posting, on_submit_comment.clone()));
                        } else {
                            parts.push(who_are_you());
                        }
                        if !roots.is_empty() {
                            let (page_rows, total_pages) = paginate_level(&roots, page);
                            let views = render_comment_rows(&page_rows, &children, &base);
                            parts.push(view! { <div class="cmt-list">{views}</div> }.into_any());
                            if total_pages > 1 {
                                parts.push(level_paginator(list_page, total_pages).into_any());
                            }
                        }
                        if comments_has_next.get() {
                            let load_more = load_more.clone();
                            let vid = version_id_render.clone();
                            parts.push(
                                view! {
                                    <button
                                        class="cmt-load-more"
                                        on:click=move |_| spawn_local(load_more(vid.clone()))
                                    >{"load more comments"}</button>
                                }
                                .into_any(),
                            );
                        }
                    }
                    CommentLevel::Comment(cid) => {
                        let target = comment_list
                            .iter()
                            .find(|c| comment_id(c) == cid)
                            .cloned();
                        match target {
                            Some(target) => {
                                let delete_url = if can_reply {
                                    Some(format!("{base}/comment/{cid}/delete"))
                                } else {
                                    None
                                };
                                parts.push(context_card(&target, delete_url));
                                if can_reply {
                                    parts.push(comment_form(reply_body, posting, on_submit_reply.clone()));
                                } else {
                                    parts.push(who_are_you());
                                }
                                let kids: Vec<&serde_json::Value> = children
                                    .get(cid.as_str())
                                    .map(|v| v.to_vec())
                                    .unwrap_or_default();
                                if !kids.is_empty() {
                                    let (page_rows, total_pages) = paginate_level(&kids, page);
                                    let views = render_comment_rows(&page_rows, &children, &base);
                                    parts.push(view! { <div class="cmt-list">{views}</div> }.into_any());
                                    if total_pages > 1 {
                                        parts.push(level_paginator(list_page, total_pages).into_any());
                                    }
                                }
                            }
                            None => {
                                parts.push(
                                    view! { <p class="cmt-empty">comment not found</p> }.into_any(),
                                );
                            }
                        }
                    }
                    CommentLevel::DeleteComment(cid) => {
                        let target_exists = comment_list
                            .iter()
                            .any(|c| comment_id(c) == cid);
                        match target_exists {
                            true => {
                                if !can_reply {
                                    parts.push(who_are_you());
                                } else if delete_denied.get() && delete_checked.get() {
                                    parts.push(denied_view());
                                } else if !delete_checked.get() {
                                    parts.push(
                                        view! { <p class="cmt-empty">loading...</p> }.into_any(),
                                    );
                                } else {
                                    parts.push(delete_comment_form(
                                        posting,
                                        on_submit_delete_comment,
                                    ));
                                }
                            }
                            false => {
                                parts.push(
                                    view! { <p class="cmt-empty">comment not found</p> }.into_any(),
                                );
                            }
                        }
                    }
                }

                view! { <div>{parts}</div> }.into_any()
            }}
        </section>
    }
}
