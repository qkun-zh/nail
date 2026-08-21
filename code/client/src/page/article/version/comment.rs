pub mod delete;
pub mod detail;
pub mod index;
pub mod pagination;
pub mod render;
pub mod state;
pub mod undelete;
pub mod update;
pub mod url;

use delete::comment_delete_view;
use detail::comment_detail_view;
use index::comment_list_view;
use undelete::comment_undelete_view;
use update::comment_update_view;

use common::request::DeleteMode;
use common::response::ListPage;
use common::response::comment::CommentView;
use leptos::prelude::*;
use leptos_router::hooks::{query_signal, use_navigate, use_params_map, use_query_map};

use crate::infrastructure::limits::use_limits;
use crate::page::notify::use_notifications;
use crate::page::session_gate::who_are_you;
use crate::page::validation::validate_uuid;

use render::{CommentViewContext, STYLE};
use state::{
    CommentSignals, build_load, build_submit_comment, build_submit_delete, build_submit_reply,
    build_submit_undelete, build_submit_update,
};
use url::{CommentLevel, comment_level_from_path};
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
    let base = move || format!("/article/{}/version/{}", article_id(), version_id_param());

    let roots = RwSignal::new(None::<ListPage<CommentView>>);
    let target = RwSignal::new(None::<CommentView>);
    let children = RwSignal::new(None::<ListPage<CommentView>>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(true);
    let posting = RwSignal::new(false);
    let delete_mode = RwSignal::new(DeleteMode::Transfer);
    let body = RwSignal::new(query.get_untracked().get("body").unwrap_or_default());
    let reply_body = RwSignal::new(query.get_untracked().get("reply").unwrap_or_default());
    let update_body = RwSignal::new(query.get_untracked().get("update").unwrap_or_default());

    crate::page::draft::sync_url_on_change(navigate.clone(), move || {
        let _ = (
            body.get(),
            reply_body.get(),
            update_body.get(),
            comment_path(),
            page.get(),
        );
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
            CommentLevel::UpdateComment(comment_id) => {
                let value = update_body.get();
                if !value.trim().is_empty() {
                    pairs.push(("update".to_string(), value));
                }
                format!("{base_path}/comment/{comment_id}/update")
            }
            CommentLevel::DeleteComment(_)
            | CommentLevel::UndeleteComment(_)
            | CommentLevel::Invalid => return None,
        };
        pairs.push(("page".to_string(), page_value.to_string()));
        let refs: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        Some(crate::page::draft::draft_url(&pathname, &refs))
    });

    let load = StoredValue::new(build_load(
        notifications.clone(),
        CommentSignals {
            loading,
            roots,
            target,
            children,
            error,
        },
        comment_path,
        move || page.get(),
    ));

    Effect::new(move |_| {
        let _ = page.get();
        let _ = comment_path();
        let version_id = version_id_param();
        let load = load.get_value();
        if let Err(error_message) = validate_uuid(&version_id) {
            error.set(Some(error_message));
            return;
        }
        load(version_id);
    });

    let on_submit_comment = build_submit_comment(
        notifications.clone(),
        posting,
        body,
        limits,
        version_id_param,
        load.get_value(),
    );
    let on_submit_reply = build_submit_reply(
        notifications.clone(),
        posting,
        reply_body,
        limits,
        mode,
        version_id_param,
        load.get_value(),
    );
    let on_submit_update = build_submit_update(
        notifications.clone(),
        posting,
        update_body,
        limits,
        mode,
        version_id_param,
        base,
        navigate.clone(),
        load.get_value(),
    );
    let on_submit_undelete = build_submit_undelete(
        notifications.clone(),
        posting,
        mode,
        version_id_param,
        base,
        navigate.clone(),
        load.get_value(),
    );
    let on_submit_delete = build_submit_delete(
        notifications,
        posting,
        mode,
        version_id_param,
        base,
        navigate,
        load.get_value(),
    );

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
                let comment_view_context = CommentViewContext {
                    base_path,
                    current_page,
                    authenticated,
                    max_chars: limits.get().max_comment_body_chars,
                    posting,
                };
                match mode() {
                    CommentLevel::VersionComments => comment_list_view(
                        roots,
                        &comment_view_context,
                        body,
                        on_submit_comment.clone(),
                    )
                    .into_any(),
                    CommentLevel::Comment(comment_id) => comment_detail_view(
                        target,
                        children,
                        &comment_id,
                        &comment_view_context,
                        reply_body,
                        on_submit_reply.clone(),
                    )
                    .into_any(),
                    CommentLevel::DeleteComment(_) => {
                        if !authenticated {
                            return who_are_you();
                        }
                        comment_delete_view(delete_mode, comment_view_context.posting, on_submit_delete.clone())
                            .into_any()
                    }
                    CommentLevel::UpdateComment(_) => {
                        if !authenticated {
                            return who_are_you();
                        }
                        comment_update_view(
                            update_body,
                            comment_view_context.posting,
                            comment_view_context.max_chars,
                            on_submit_update.clone(),
                        )
                        .into_any()
                    }
                    CommentLevel::UndeleteComment(_) => {
                        if !authenticated {
                            return who_are_you();
                        }
                        comment_undelete_view(
                            comment_view_context.posting,
                            on_submit_undelete.clone(),
                        )
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
