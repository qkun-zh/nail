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
use leptos_router::NavigateOptions;
use leptos_router::hooks::{
    query_signal, query_signal_with_options, use_navigate, use_params_map, use_query_map,
};

use crate::infrastructure::limits::use_limits;
use crate::page::fetch::{LoadError, Loaded};
use crate::page::notify::use_notifications;
use crate::page::session_gate::who_are_you;
use crate::page::validation::validate_uuid;

use pagination::COMMENTS_PER_PAGE;
use render::{CommentViewContext, STYLE};
use state::{
    build_submit_comment, build_submit_delete, build_submit_reply, build_submit_undelete,
    build_submit_update,
};
use url::{CommentLevel, comment_level_from_path};

#[derive(Clone)]
enum CommentLoad {
    Roots(ListPage<CommentView>),
    Detail {
        target: Box<CommentView>,
        children: ListPage<CommentView>,
    },
    Blank,
}

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

    // Draft textareas seeded once from the URL and mirrored back through the router.
    let body = RwSignal::new(query.get_untracked().get("body").unwrap_or_default());
    let reply_body = RwSignal::new(query.get_untracked().get("reply").unwrap_or_default());
    let update_body = RwSignal::new(query.get_untracked().get("update").unwrap_or_default());
    for (key, source) in [
        ("body", body),
        ("reply", reply_body),
        ("update", update_body),
    ] {
        let options = NavigateOptions {
            replace: true,
            ..Default::default()
        };
        let (_, set_param) = query_signal_with_options::<String>(key, options);
        Effect::new(move |_| {
            let value = source.get();
            set_param.set((!value.trim().is_empty()).then_some(value));
        });
    }

    let comments: LocalResource<Loaded<CommentLoad>> = LocalResource::new(move || {
        let version_id = version_id_param();
        let level = mode();
        let page_value = page.get();
        async move {
            validate_uuid(&version_id).map_err(LoadError::from)?;
            match level {
                CommentLevel::VersionComments => Ok(CommentLoad::Roots(
                    crate::request::comment::read_comments(
                        &version_id,
                        page_value,
                        COMMENTS_PER_PAGE,
                    )
                    .await?,
                )),
                CommentLevel::Comment(comment_id) => {
                    let target = crate::request::comment::read_comment(&comment_id).await?;
                    let children = crate::request::comment::read_comment_children(
                        &comment_id,
                        page_value,
                        COMMENTS_PER_PAGE,
                    )
                    .await?;
                    Ok(CommentLoad::Detail {
                        target: Box::new(target),
                        children,
                    })
                }
                CommentLevel::DeleteComment(_)
                | CommentLevel::UpdateComment(_)
                | CommentLevel::UndeleteComment(_) => Ok(CommentLoad::Blank),
                CommentLevel::Invalid => Err(LoadError::from("comment not found".to_string())),
            }
        }
    });

    let delete_mode = RwSignal::new(DeleteMode::Transfer);
    let submit = state::CommentSubmit {
        notifications: notifications.clone(),
        posting: RwSignal::new(false),
        limits,
    };
    let posting = submit.posting;
    let reload = move || comments.refetch();

    let on_submit_comment = build_submit_comment(submit.clone(), body, version_id_param, reload);
    let on_submit_reply = build_submit_reply(submit.clone(), reply_body, mode, reload);
    let on_submit_update = build_submit_update(
        submit.clone(),
        update_body,
        mode,
        base,
        navigate.clone(),
        reload,
    );
    let on_submit_undelete =
        build_submit_undelete(submit.clone(), mode, base, navigate.clone(), reload);
    let on_submit_delete = build_submit_delete(submit, mode, base, navigate, reload);

    view! {
        <section class="cmt-section">
            <style>{STYLE}</style>
            {move || match comments.get() {
                None => view! { <p class="cmt-loading">loading comments...</p> }.into_any(),
                Some(Err(message)) => view! { <p class="cmt-empty">{message.to_string()}</p> }.into_any(),
                Some(Ok(load)) => {
                    let authenticated = !crate::request::session::read_session_token()
                        .unwrap_or_default()
                        .is_empty();
                    let comment_view_context = CommentViewContext {
                        base_path: base(),
                        current_page: page.get(),
                        authenticated,
                        max_chars: limits.get().max_comment_body_chars,
                        posting,
                    };
                    match (mode(), load) {
                        (CommentLevel::VersionComments, CommentLoad::Roots(roots)) => {
                            comment_list_view(&roots, &comment_view_context, body, on_submit_comment.clone())
                                .into_any()
                        }
                        (
                            CommentLevel::Comment(comment_id),
                            CommentLoad::Detail { target, children },
                        ) => comment_detail_view(
                            &target,
                            &children,
                            &comment_id,
                            &comment_view_context,
                            reply_body,
                            on_submit_reply.clone(),
                        )
                        .into_any(),
                        (CommentLevel::DeleteComment(_), _) => {
                            if !authenticated {
                                return who_are_you();
                            }
                            comment_delete_view(
                                delete_mode,
                                comment_view_context.posting,
                                on_submit_delete.clone(),
                            )
                            .into_any()
                        }
                        (CommentLevel::UpdateComment(_), _) => {
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
                        (CommentLevel::UndeleteComment(_), _) => {
                            if !authenticated {
                                return who_are_you();
                            }
                            comment_undelete_view(
                                comment_view_context.posting,
                                on_submit_undelete.clone(),
                            )
                            .into_any()
                        }
                        _ => view! { <p class="cmt-loading">loading comments...</p> }.into_any(),
                    }
                }
            }}
        </section>
    }
}
