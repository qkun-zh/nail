use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};
use nail_common::response::search::{SearchArticleItem, SearchCommentItem, SearchVersionItem};

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, use_notifications};
use crate::page::pagination::{LocalPagedList, Pagination};
use crate::page::public::article::version::comment::pagination::COMMENTS_PER_PAGE;
use crate::request::url::encode_component;

const VERSIONS_PER_PAGE: u64 = 8;

const STYLE: &str = r#"
:root {
  --bg: #f2f4f7;
  --card: #ffffff;
  --ink: #1a1d21;
  --muted: #6b7280;
  --faint: #9aa1ab;
  --line: #e5e7eb;
  --line-strong: #d1d5db;
  --accent: #2563eb;
  --accent-soft: #eff6ff;
  --mark-bg: #fde047;
  --mark-fg: #422006;
  --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

* { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--bg);
  color: var(--ink);
  font: 15px/1.6 -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
  padding: 0 0 80px;
}

.searchbar {
  position: sticky;
  top: 0;
  background: var(--bg);
  border-bottom: 1px solid var(--line);
  padding: 18px 24px;
  z-index: 10;
}
.searchbar-inner {
  max-width: 860px;
  margin: 0 auto;
}
.searchbar .query-row {
  display: flex;
  gap: 10px;
}
.searchbar input[type="text"] {
  flex: 1;
  font-size: 16px;
  padding: 11px 14px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  outline: none;
  background: #fff;
}
.searchbar input:focus { border-color: var(--accent); }
.searchbar .go {
  font-size: 15px;
  padding: 0 22px;
  border: none;
  border-radius: 10px;
  background: var(--accent);
  color: #fff;
  cursor: pointer;
}
.searchbar .controls {
  display: flex;
  gap: 20px;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 12px;
  font-size: 13px;
  color: var(--ink);
}
.searchbar .controls .group { display: flex; gap: 8px; align-items: center; }
.searchbar .controls .group-title { font-size: 12px; color: var(--muted); }
.searchbar .controls label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}
.searchbar .controls input[type="checkbox"] { accent-color: var(--accent); }
.searchbar .controls input[type="text"] {
  width: 190px;
  font-size: 13px;
  padding: 6px 10px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  outline: none;
  background: #fff;
}
.searchbar .sort-btn {
  font-size: 13px;
  padding: 5px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: #fff;
  color: var(--ink);
  cursor: pointer;
}
.searchbar .sort-btn:hover { border-color: var(--accent); color: var(--accent); }
.searchbar .sort-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  padding: 5px 10px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--ink);
}
.searchbar .sort-chip .dir { cursor: pointer; }
.searchbar .sort-chip .rm { cursor: pointer; color: var(--muted); }
.searchbar .sort-chip .rm:hover { color: #dc2626; }

.wrap {
  max-width: 860px;
  margin: 0 auto;
  padding: 20px 24px;
}

mark {
  background: var(--mark-bg);
  color: var(--mark-fg);
  border-radius: 2px;
  padding: 0 1px;
}

/* ============ article ============ */
.article {
  background: var(--card);
  border: 1px solid var(--line-strong);
  border-radius: 14px;
  margin-bottom: 20px;
  box-shadow: 0 1px 3px rgba(16,24,40,.06);
  overflow: hidden;
}

.article-head {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  padding: 16px 20px;
  border-bottom: 1px solid var(--line);
  background: #fbfcfd;
}
.article-head .label-chip {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: .04em;
  text-transform: uppercase;
  color: var(--muted);
  text-decoration: none;
}
.article-head .label-chip:hover { color: var(--accent); }
.article-head .label-chip .dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
}
.article-head .title {
  font-size: 15px;
  font-weight: normal;
  color: var(--ink);
}
.article-head .meta { color: var(--ink); font-size: 15px; }

.hits { padding: 14px 20px; }

/* ============ field cards ============ */
.field-card {
  border: 1px solid var(--line);
  border-radius: 10px;
  background: #fff;
  margin-bottom: 12px;
  overflow: hidden;
}
.field-card:last-child { margin-bottom: 0; }

.field-label {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: .04em;
  text-transform: uppercase;
  color: var(--muted);
  background: #f8fafc;
  border-bottom: 1px solid var(--line);
}
.field-label .dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
}
.field-label .version-link {
  color: var(--muted);
  text-decoration: none;
}
.field-label .version-link:hover { color: var(--accent); }
.field-label .version-chip {
  font-size: 13px;
  font-weight: normal;
  text-transform: none;
  letter-spacing: 0;
  color: var(--ink);
}
.field-label .version-time {
  font-size: 13px;
  font-weight: normal;
  text-transform: none;
  letter-spacing: 0;
  color: var(--ink);
}
.field-body { padding: 12px 14px; }

/* ============ comment cards ============ */
.comment-hit {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #fafbfc;
  padding: 10px 12px;
  margin-bottom: 8px;
}
.comment-hit:last-child { margin-bottom: 0; }

.cmt-main { flex: 1; }
.cmt-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 3px;
}
.cmt-author {
  font-weight: normal;
  font-size: 13px;
  color: var(--ink);
  text-decoration: none;
}
.cmt-author:hover { color: var(--accent); }
.cmt-author:hover .cmt-time { color: var(--accent); }
.cmt-time { color: var(--ink); font-size: 13px; }
.cmt-content { font-size: 14px; color: var(--ink); }

.comment-head-row { display: flex; align-items: flex-start; gap: 8px; }

/* ============ pagination ============ */
.pagination {
  display: flex;
  gap: 8px;
  justify-content: center;
  align-items: center;
  margin-top: 24px;
  color: var(--muted);
  font-size: 14px;
}
.pagination button {
  padding: 6px 14px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: #fff;
  color: var(--ink);
  font-size: 14px;
  cursor: pointer;
}
.pagination button:disabled {
  color: var(--faint);
  cursor: not-allowed;
}
.pagination button:not(:disabled):hover { border-color: var(--accent); color: var(--accent); }
.pagination input {
  width: 52px;
  padding: 6px 8px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  text-align: center;
  font-size: 14px;
  color: var(--ink);
  background: #fff;
  outline: none;
}
.pagination input:focus { border-color: var(--accent); }
.pagination .total { color: var(--muted); }

.comment-pagination {
  display: flex;
  gap: 6px;
  align-items: center;
  justify-content: center;
  margin-top: 10px;
  color: var(--muted);
  font-size: 12px;
}
.comment-pagination button {
  padding: 3px 10px;
  border: 1px solid var(--line-strong);
  border-radius: 6px;
  background: #fff;
  color: var(--ink);
  font-size: 12px;
  cursor: pointer;
}
.comment-pagination button:disabled {
  color: var(--faint);
  cursor: not-allowed;
}
.comment-pagination button:not(:disabled):hover { border-color: var(--accent); color: var(--accent); }
.comment-pagination input {
  width: 44px;
  padding: 3px 6px;
  border: 1px solid var(--line-strong);
  border-radius: 6px;
  text-align: center;
  font-size: 12px;
  color: var(--ink);
  background: #fff;
  outline: none;
}
.comment-pagination input:focus { border-color: var(--accent); }
.comment-pagination .total { color: var(--muted); }
"#;

const RANGE_KEYS: [&str; 7] = [
    "title",
    "summary",
    "author_name",
    "comment",
    "note",
    "tag",
    "version_number",
];
const RANGE_LABELS: [&str; 7] = [
    "title",
    "summary",
    "author name",
    "comment",
    "version note",
    "tag",
    "version number",
];
const SORT_KEYS: [&str; 3] = ["time", "title", "author"];
const SEARCH_PATHNAME: &str = "/public/article/search";

fn normalize_iso8601(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let mut normalized = value.trim().to_string();
    let has_timezone = normalized.ends_with('Z')
        || normalized.ends_with('z')
        || normalized
            .rfind(['+', '-'])
            .is_some_and(|index| normalized.find('T').is_some_and(|t_index| index > t_index));
    if !has_timezone {
        normalized.push('Z');
    }
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from(normalized.clone()));
    if date.get_time().is_nan() {
        return None;
    }
    Some(normalized)
}

fn sort_label(key: &str) -> &str {
    match key {
        "time" => "time",
        "title" => "title",
        "author" => "author",
        _ => key,
    }
}

fn default_sort_dir(key: &str) -> String {
    if key == "time" {
        "desc".to_string()
    } else {
        "asc".to_string()
    }
}

fn dir_arrow(dir: &str) -> &'static str {
    if dir == "desc" { "↓" } else { "↑" }
}

/// A version's comment list with client-side pagination. The search response
/// already carries all comments for a version, so paging is local state only.
#[component]
fn SearchComments(
    article_id: String,
    version_id: String,
    comments: Vec<SearchCommentItem>,
) -> impl IntoView {
    let render = move |comment: &SearchCommentItem| {
        let comment_url = format!(
            "/public/article/{article_id}/version/{version_id}/comment/{}",
            comment.comment_id
        );
        let author_html = comment.author_name.clone();
        let time_text = comment.time.clone();
        let content_html = comment.content.clone();
        view! {
            <div class="comment-hit">
                <div class="comment-head-row">
                    <div class="cmt-main">
                        <div class="cmt-meta">
                            <A attr:class="cmt-author" href=comment_url>
                                <span inner_html=author_html></span>
                                <span class="cmt-time">{time_text}</span>
                            </A>
                        </div>
                        <div class="cmt-content" inner_html=content_html></div>
                    </div>
                </div>
            </div>
        }
        .into_any()
    };
    view! {
        <div class="field-card">
            <div class="field-label"><span class="dot"></span>comment</div>
            <div class="field-body">
                <LocalPagedList
                    items=comments
                    per_page=COMMENTS_PER_PAGE
                    pagination_class="comment-pagination"
                    render=render
                />
            </div>
        </div>
    }
}

/// An article's hit-version list with client-side pagination. The search
/// response already carries all hit versions for an article, so paging is
/// local state only (refreshing the page returns to the first page).
#[component]
fn SearchVersions(article_id: String, versions: Vec<SearchVersionItem>) -> impl IntoView {
    let render = move |version: &SearchVersionItem| {
        let version_url = format!(
            "/public/article/{}/version/{}",
            article_id, version.version_id
        );
        let version_chip_html = version.version_number.clone();
        let version_time_text = version.time.clone();
        let version_hits = version.version_hits.clone();
        let comments = version.comments.clone();
        let article_id_for_comments = article_id.clone();
        let version_id_for_comments = version.version_id.clone();
        let show_comments = !comments.is_empty();
        view! {
            <div class="field-card">
                <div class="field-label">
                    <span class="dot"></span>
                    <A attr:class="version-link" href=version_url>version</A>
                    <span class="version-chip" inner_html=version_chip_html></span>
                    <span class="version-time">{version_time_text}</span>
                </div>
                <div class="field-body">
                    {version_hits
                        .into_iter()
                        .map(|hit| {
                            let label = hit.label.clone();
                            let snippet = hit.snippet.clone();
                            view! {
                                <div class="field-card">
                                    <div class="field-label"><span class="dot"></span>{label}</div>
                                    <div class="field-body" inner_html=snippet></div>
                                </div>
                            }
                        })
                        .collect_view()}
                    {show_comments
                        .then(|| {
                            view! {
                                <SearchComments
                                    article_id=article_id_for_comments
                                    version_id=version_id_for_comments
                                    comments=comments
                                />
                            }
                        })}
                </div>
            </div>
        }
        .into_any()
    };
    view! {
        <LocalPagedList
            items=versions
            per_page=VERSIONS_PER_PAGE
            pagination_class="comment-pagination"
            render=render
        />
    }
}

#[component]
pub fn Search() -> impl IntoView {
    let notifications = use_notifications();
    let navigate = use_navigate();
    let query = use_query_map();
    let limits = use_limits();

    let search_list = RwSignal::new(Vec::<SearchArticleItem>::new());
    let loaded = RwSignal::new(false);
    let fetching = RwSignal::new(false);
    let total = RwSignal::new(0u64);
    let total_pages = RwSignal::new(0u64);
    let truncated = RwSignal::new(false);

    let q_filter = RwSignal::new(String::new());
    let ranges = RwSignal::new(vec![true; 7]);
    let from_time = RwSignal::new(String::new());
    let to_time = RwSignal::new(String::new());
    let sort_order = RwSignal::new(Vec::<(String, String)>::new());
    let current_page = RwSignal::new(1u64);
    let per_page = RwSignal::new(limits.get_untracked().search_page_size);

    let params = query.get_untracked();
    q_filter.set(params.get("q").unwrap_or_default());
    if let Some(ranges_param) = params.get("ranges") {
        let mut checked = vec![false; 7];
        if !ranges_param.is_empty() {
            for (index, key) in RANGE_KEYS.iter().enumerate() {
                if ranges_param.split(',').any(|piece| piece == *key) {
                    checked[index] = true;
                }
            }
        }
        ranges.set(checked);
    }
    if let Some(sort_param) = params.get("sort") {
        let mut order = Vec::new();
        for piece in sort_param.split(',') {
            let mut parts = piece.splitn(2, ':');
            let key = parts.next().unwrap_or("");
            let default_dir = default_sort_dir(key);
            let direction = parts.next().unwrap_or(&default_dir);
            if SORT_KEYS.contains(&key) {
                order.push((key.to_string(), direction.to_string()));
            }
        }
        sort_order.set(order);
    }
    from_time.set(params.get("from").unwrap_or_default());
    to_time.set(params.get("to").unwrap_or_default());
    let page = params
        .get("page")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    current_page.set(page);
    if let Some(limit) = params
        .get("limit")
        .and_then(|value| value.parse::<u64>().ok())
    {
        per_page.set(limit);
    }

    let sync_url = {
        let navigate = navigate.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            let q = q_filter.get();
            if !q.trim().is_empty() {
                pairs.push(format!("q={}", encode_component(q.trim())));
            }
            let checked = ranges.get();
            let subset = RANGE_KEYS
                .iter()
                .enumerate()
                .filter(|(index, _)| checked[*index])
                .map(|(_, key)| *key)
                .collect::<Vec<_>>()
                .join(",");
            pairs.push(format!("ranges={}", encode_component(&subset)));
            let order = sort_order.get();
            if !order.is_empty() {
                let serialized = order
                    .iter()
                    .map(|(key, direction)| format!("{key}:{direction}"))
                    .collect::<Vec<_>>()
                    .join(",");
                pairs.push(format!("sort={}", encode_component(&serialized)));
            }
            let from = from_time.get();
            if !from.trim().is_empty() {
                pairs.push(format!("from={}", encode_component(from.trim())));
            }
            let to = to_time.get();
            if !to.trim().is_empty() {
                pairs.push(format!("to={}", encode_component(to.trim())));
            }
            pairs.push(format!("page={}", current_page.get()));
            let query_string = pairs.join("&");
            navigate(
                &format!("{SEARCH_PATHNAME}?{query_string}"),
                NavigateOptions {
                    replace: true,
                    resolve: false,
                    ..Default::default()
                },
            );
        }
    };

    let request_seq = StoredValue::new(0u64);
    let last_good_page = StoredValue::new(1u64);
    let search_notifications = notifications.clone();
    let run_search = move |pairs: Vec<(String, String)>| {
        let my_seq = request_seq.get_value() + 1;
        request_seq.set_value(my_seq);
        fetching.set(true);
        let notifications = search_notifications.clone();
        leptos::task::spawn_local(async move {
            let borrows: Vec<(&str, &str)> = pairs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect();
            match crate::request::article::search_articles(&borrows).await {
                Ok(page) => {
                    if request_seq.get_value() != my_seq {
                        return;
                    }
                    search_list.set(page.article_list);
                    total.set(page.total);
                    total_pages.set(page.total_pages);
                    truncated.set(page.truncated);
                    let committed = if page.total_pages > 0 {
                        page.page.min(page.total_pages)
                    } else {
                        page.page
                    };
                    current_page.set(committed);
                    last_good_page.set_value(committed);
                }
                Err(error) => {
                    if request_seq.get_value() == my_seq {
                        notify_error(&notifications, error.to_string());
                        loaded.set(true);
                        fetching.set(false);
                        current_page.set(last_good_page.get_value());
                    }
                    return;
                }
            }
            loaded.set(true);
            fetching.set(false);
        });
    };

    let do_search = {
        let run_search = run_search.clone();
        move |page: u64| {
            let mut pairs: Vec<(String, String)> = vec![
                ("page".to_string(), page.to_string()),
                ("limit".to_string(), per_page.get_untracked().to_string()),
            ];
            let q = q_filter.get_untracked().trim().to_string();
            if q.is_empty() {
                search_list.set(Vec::new());
                total.set(0);
                total_pages.set(0);
                truncated.set(false);
                current_page.set(1);
                loaded.set(true);
                fetching.set(false);
                return;
            }
            pairs.push(("q".to_string(), q));
            let checked = ranges.get_untracked();
            let subset = RANGE_KEYS
                .iter()
                .enumerate()
                .filter(|(index, _)| checked[*index])
                .map(|(_, key)| *key)
                .collect::<Vec<_>>()
                .join(",");
            pairs.push(("ranges".to_string(), subset));
            let order = sort_order.get_untracked();
            if !order.is_empty() {
                let serialized = order
                    .iter()
                    .map(|(key, direction)| format!("{key}:{direction}"))
                    .collect::<Vec<_>>()
                    .join(",");
                pairs.push(("sort".to_string(), serialized));
            }
            let from = from_time.get_untracked();
            if !from.trim().is_empty() {
                pairs.push(("from".to_string(), from.trim().to_string()));
            }
            let to = to_time.get_untracked();
            if !to.trim().is_empty() {
                pairs.push(("to".to_string(), to.trim().to_string()));
            }
            run_search(pairs);
        }
    };

    let trigger_search = {
        let do_search = do_search.clone();
        move || {
            current_page.set(1);
            do_search(1);
        }
    };
    let on_submit = {
        let trigger_search = trigger_search.clone();
        move |event: SubmitEvent| {
            event.prevent_default();
            trigger_search();
        }
    };
    let on_range_change = {
        let trigger_search = trigger_search.clone();
        move |index: usize, event: web_sys::Event| {
            ranges.update(|checked| checked[index] = event_target_checked(&event));
            trigger_search();
        }
    };
    let on_add_sort = {
        let trigger_search = trigger_search.clone();
        move |key: String| {
            sort_order.update(|order| {
                if !order.iter().any(|(sort_key, _)| *sort_key == key) {
                    order.push((key.clone(), default_sort_dir(&key)));
                }
            });
            trigger_search();
        }
    };
    let on_toggle_dir = {
        let trigger_search = trigger_search.clone();
        move |key: String| {
            sort_order.update(|order| {
                if let Some(entry) = order.iter_mut().find(|(sort_key, _)| *sort_key == key) {
                    entry.1 = if entry.1 == "asc" {
                        "desc".to_string()
                    } else {
                        "asc".to_string()
                    };
                }
            });
            trigger_search();
        }
    };
    let on_remove_sort = {
        let trigger_search = trigger_search.clone();
        move |key: String| {
            sort_order.update(|order| order.retain(|(sort_key, _)| *sort_key != key));
            trigger_search();
        }
    };
    let on_from_change = {
        let trigger_search = trigger_search.clone();
        let notifications = notifications.clone();
        move |event: web_sys::Event| {
            let value = event_target_value(&event);
            if value.is_empty() || normalize_iso8601(&value).is_some() {
                from_time.set(value);
                trigger_search();
            } else {
                notify_error(
                    &notifications,
                    "from must be ISO8601 (e.g. 2024-01-15T10:30:00, no timezone = UTC)",
                );
            }
        }
    };
    let on_to_change = {
        let trigger_search = trigger_search.clone();
        let notifications = notifications.clone();
        move |event: web_sys::Event| {
            let value = event_target_value(&event);
            if value.is_empty() || normalize_iso8601(&value).is_some() {
                to_time.set(value);
                trigger_search();
            } else {
                notify_error(
                    &notifications,
                    "to must be ISO8601 (e.g. 2024-01-15T10:30:00, no timezone = UTC)",
                );
            }
        }
    };

    let on_go = Callback::new({
        let do_search = do_search.clone();
        move |target: u64| {
            if target == current_page.get() {
                return;
            }
            current_page.set(target);
            do_search(target);
        }
    });

    Effect::new(move |previous: Option<()>| {
        let _ = (
            q_filter.get(),
            ranges.get(),
            from_time.get(),
            to_time.get(),
            sort_order.get(),
            current_page.get(),
        );
        if previous.is_none() {
            return;
        }
        sync_url();
    });

    do_search.clone()(page);

    view! {
        <style>{STYLE}</style>
        <div class="searchbar">
            <div class="searchbar-inner">
                <form on:submit=on_submit>
                    <div class="query-row">
                        <input
                            type="text"
                            placeholder="search text (space separated words = AND)"
                            prop:value=q_filter
                            on:input=move |event| q_filter.set(event_target_value(&event))
                        />
                        <button type="submit" class="go" disabled=move || fetching.get()>search</button>
                    </div>
                    <div class="controls">
                        <div class="group">
                            <span class="group-title">ranges</span>
                            {RANGE_LABELS
                                .iter()
                                .enumerate()
                                .map(|(index, label)| {
                                    let handler = {
                                        let on_range_change = on_range_change.clone();
                                        move |event: web_sys::Event| on_range_change(index, event)
                                    };
                                    let checked = move || ranges.get()[index];
                                    view! {
                                        <label>
                                            <input type="checkbox" prop:checked=checked on:change=handler/>
                                            {*label}
                                        </label>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <div class="group">
                            <span class="group-title">time</span>
                            <input
                                type="text"
                                placeholder="from (ISO8601, UTC)"
                                prop:value=from_time
                                on:change=on_from_change
                            />
                            <input
                                type="text"
                                placeholder="to (ISO8601, UTC)"
                                prop:value=to_time
                                on:change=on_to_change
                            />
                        </div>
                        <div class="group">
                            <span class="group-title">sort</span>
                            {SORT_KEYS
                                .iter()
                                .map(|key| {
                                    let on_add_sort = on_add_sort.clone();
                                    let key = key.to_string();
                                    let label = sort_label(&key).to_string();
                                    view! {
                                        <button type="button" class="sort-btn" on:click=move |_| on_add_sort(key.clone())>
                                            {label}
                                        </button>
                                    }
                                })
                                .collect_view()}
                            {move || {
                                let order = sort_order.get();
                                order
                                    .into_iter()
                                    .map(|(key, direction)| {
                                        let on_toggle_dir = on_toggle_dir.clone();
                                        let on_remove_sort = on_remove_sort.clone();
                                        let toggle_key = key.clone();
                                        let remove_key = key.clone();
                                        let label = sort_label(&key).to_string();
                                        let arrow = dir_arrow(&direction).to_string();
                                        view! {
                                            <span class="sort-chip">
                                                <span class="dir" on:click=move |_| on_toggle_dir(toggle_key.clone())>
                                                    {arrow}
                                                </span>
                                                {label}
                                                <span class="rm" on:click=move |_| on_remove_sort(remove_key.clone())>
                                                    {"×"}
                                                </span>
                                            </span>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                    </div>
                </form>
            </div>
        </div>
        <div class="wrap">
            {move || {
                if truncated.get() {
                    let message = format!(
                        "too many results ({} records) - only the first {} pages are shown, add more conditions to narrow down",
                        total.get(),
                        limits.get().max_search_pages
                    );
                    view! { <p>{message}</p> }.into_any()
                } else {
                    ().into_any()
                }
            }}
            {move || {
                if loaded.get() {
                    let list = search_list.get();
                    if list.is_empty() {
                        if q_filter.get_untracked().trim().is_empty() {
                            view! { <p class="empty-hint">enter a query to search</p> }.into_any()
                        } else {
                            view! { <p>none</p> }.into_any()
                        }
                    } else {
                        let rows = list
                            .into_iter()
                            .map(|article| {
                                let detail_url = format!("/public/article/{}", article.article_id);
                                let title_html = article.title.clone();
                                let author_html = article.author_name.clone();
                                let time_text = article.time.clone();
                                let article_hits = article.article_hits.clone();
                                let versions = article.versions.clone();
                                view! {
                                    <div class="article">
                                        <div class="article-head">
                                            <A attr:class="label-chip" href=detail_url>
                                                <span class="dot"></span>
                                                {"article"}
                                            </A>
                                            <span class="title" inner_html=title_html></span>
                                            <span class="meta">
                                                <span inner_html=author_html></span>
                                                {format!(" · {time_text}")}
                                            </span>
                                        </div>
                                        <div class="hits">
                                            {article_hits
                                                .into_iter()
                                                .map(|hit| {
                                                    let label = hit.label.clone();
                                                    let snippet = hit.snippet.clone();
                                                    view! {
                                                        <div class="field-card">
                                                            <div class="field-label"><span class="dot"></span>{label}</div>
                                                            <div class="field-body" inner_html=snippet></div>
                                                        </div>
                                                    }
                                                })
                                                .collect_view()}
                                            <SearchVersions
                                                article_id=article.article_id.clone()
                                                versions=versions
                                            />
                                        </div>
                                    </div>
                                }
                            })
                            .collect_view();
                        view! {
                            <div>
                                {rows}
                                <Pagination
                                    current=move || current_page.get()
                                    total_pages=move || total_pages.get()
                                    on_go=on_go
                                />
                            </div>
                        }
                        .into_any()
                    }
                } else {
                    view! { <p>loading...</p> }.into_any()
                }
            }}
        </div>
    }
}

#[cfg(test)]
#[path = "../../../../../../test/unit/front/page/public/article/search/tests.rs"]
mod tests;
