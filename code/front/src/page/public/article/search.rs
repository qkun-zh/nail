use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};
use nail_common::response::article::ArticleListPage;
use nail_common::response::search::{SearchArticleItem, SearchPage};

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, use_notifications};
use crate::page::pagination::Pagination;
use crate::request::url::encode_component;

const RANGE_KEYS: [&str; 6] = ["title", "summary", "author", "comment", "note", "tag"];
const RANGE_LABELS: [&str; 6] = [
    "title",
    "summary",
    "author",
    "comment",
    "version note",
    "tag",
];
const SORT_KEYS: [&str; 3] = ["time", "title", "author"];
const SEARCH_PATHNAME: &str = "/public/article/search";

#[derive(Clone, Copy, PartialEq, Eq)]
enum PageMode {
    List,
    Search,
}

enum PageOutcome {
    List(ArticleListPage),
    Search(SearchPage),
}

fn datetime_local_to_epoch_secs(value: &str) -> Option<u64> {
    if value.is_empty() {
        return None;
    }
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from(value));
    let millis = date.get_time();
    if millis.is_nan() || millis < 0.0 {
        return None;
    }
    Some((millis / 1000.0) as u64)
}

fn epoch_secs_to_datetime_local(secs: u64) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(secs as f64 * 1000.0));
    let year = date.get_full_year();
    let month = date.get_month() + 1;
    let day = date.get_date();
    let hour = date.get_hours();
    let minute = date.get_minutes();
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}")
}

fn url_time_to_local(value: &str) -> String {
    if value.chars().all(|c| c.is_ascii_digit())
        && let Ok(secs) = value.parse::<u64>()
    {
        return epoch_secs_to_datetime_local(secs);
    }
    value.to_string()
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

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn highlight_terms(escaped: &str, terms: &[String]) -> String {
    let lower = escaped.to_lowercase();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for term in terms {
        let needle = escape_html(term).to_lowercase();
        if needle.is_empty() {
            continue;
        }
        let mut start = 0;
        while let Some(pos) = lower[start..].find(&needle) {
            let abs = start + pos;
            spans.push((abs, abs + needle.len()));
            start = abs + needle.len();
        }
    }
    spans.sort_by_key(|span| span.0);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in spans {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }
    let mut out = String::new();
    let mut cursor = 0;
    for (start, end) in merged {
        out.push_str(&escaped[cursor..start]);
        out.push_str("<mark>");
        out.push_str(&escaped[start..end]);
        out.push_str("</mark>");
        cursor = end;
    }
    out.push_str(&escaped[cursor..]);
    out
}

fn render_snippet(snippet: &str, terms: &[String]) -> String {
    if terms.is_empty() {
        escape_html(snippet)
    } else {
        highlight_terms(&escape_html(snippet), terms)
    }
}

#[component]
pub fn Search() -> impl IntoView {
    let notifications = use_notifications();
    let navigate = use_navigate();
    let query = use_query_map();
    let limits = use_limits();

    let mode = RwSignal::new(PageMode::List);
    let search_list = RwSignal::new(Vec::<SearchArticleItem>::new());
    let list_page = RwSignal::new(None::<ArticleListPage>);
    let loaded = RwSignal::new(false);
    let fetching = RwSignal::new(false);
    let total = RwSignal::new(0u64);
    let total_pages = RwSignal::new(0u64);
    let truncated = RwSignal::new(false);

    let q_filter = RwSignal::new(String::new());
    let ranges = RwSignal::new(vec![true; 6]);
    let from_time = RwSignal::new(String::new());
    let to_time = RwSignal::new(String::new());
    let from_epoch = RwSignal::new(String::new());
    let to_epoch = RwSignal::new(String::new());
    let sort_order = RwSignal::new(Vec::<(String, String)>::new());
    let current_page = RwSignal::new(1u64);
    let per_page = RwSignal::new(limits.get().search_page_size);

    let params = query.get_untracked();
    q_filter.set(params.get("q").unwrap_or_default());
    if let Some(ranges_param) = params.get("ranges") {
        let mut checked = vec![false; 6];
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
    from_time.set(
        params
            .get("from")
            .map(|value| url_time_to_local(&value))
            .unwrap_or_default(),
    );
    to_time.set(
        params
            .get("to")
            .map(|value| url_time_to_local(&value))
            .unwrap_or_default(),
    );
    from_epoch.set(params.get("from").unwrap_or_default());
    to_epoch.set(params.get("to").unwrap_or_default());
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
            if !checked.iter().all(|&is_checked| is_checked) {
                let subset = RANGE_KEYS
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| checked[*index])
                    .map(|(_, key)| *key)
                    .collect::<Vec<_>>()
                    .join(",");
                pairs.push(format!("ranges={}", encode_component(&subset)));
            }
            let order = sort_order.get();
            if !order.is_empty() {
                let serialized = order
                    .iter()
                    .map(|(key, direction)| format!("{key}:{direction}"))
                    .collect::<Vec<_>>()
                    .join(",");
                pairs.push(format!("sort={}", encode_component(&serialized)));
            }
            let from = from_epoch.get();
            if !from.is_empty() {
                pairs.push(format!("from={from}"));
            }
            let to = to_epoch.get();
            if !to.is_empty() {
                pairs.push(format!("to={to}"));
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

    let is_search_active = move || {
        !q_filter.get().trim().is_empty()
            || !ranges.get().iter().all(|&is_checked| is_checked)
            || !sort_order.get().is_empty()
            || !from_epoch.get().is_empty()
            || !to_epoch.get().is_empty()
    };

    let request_seq = StoredValue::new(0u64);
    let last_good_page = StoredValue::new(1u64);
    let search_notifications = notifications.clone();
    let run_search = move |pairs: Vec<(String, String)>, is_search: bool| {
        let my_seq = request_seq.get_value() + 1;
        request_seq.set_value(my_seq);
        fetching.set(true);
        let notifications = search_notifications.clone();
        leptos::task::spawn_local(async move {
            let outcome = if is_search {
                let borrows: Vec<(&str, &str)> = pairs
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect();
                match crate::request::article::search_articles(&borrows).await {
                    Ok(page) => PageOutcome::Search(page),
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
            } else {
                let page = pairs
                    .iter()
                    .find(|(key, _)| key == "page")
                    .and_then(|(_, value)| value.parse::<u64>().ok())
                    .unwrap_or(1);
                let limit = pairs
                    .iter()
                    .find(|(key, _)| key == "limit")
                    .and_then(|(_, value)| value.parse::<u64>().ok())
                    .unwrap_or(8);
                match crate::request::article::read_articles(page, limit).await {
                    Ok(page) => PageOutcome::List(page),
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
            };
            if request_seq.get_value() != my_seq {
                return;
            }
            match outcome {
                PageOutcome::Search(page) => {
                    mode.set(PageMode::Search);
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
                PageOutcome::List(page) => {
                    mode.set(PageMode::List);
                    list_page.set(Some(page.clone()));
                    search_list.set(Vec::new());
                    total.set(page.total);
                    total_pages.set(page.total_pages);
                    truncated.set(page.truncated);
                    current_page.set(page.page);
                    last_good_page.set_value(page.page);
                }
            }
            loaded.set(true);
            fetching.set(false);
        });
    };

    let do_search = {
        let run_search = run_search.clone();
        move |page: u64| {
            let is_search = is_search_active();
            let mut pairs: Vec<(String, String)> = vec![
                ("page".to_string(), page.to_string()),
                ("limit".to_string(), per_page.get().to_string()),
            ];
            if is_search {
                let q = q_filter.get().trim().to_string();
                if !q.is_empty() {
                    pairs.push(("q".to_string(), q));
                }
                let checked = ranges.get();
                if !checked.iter().all(|&is_checked| is_checked) {
                    let subset = RANGE_KEYS
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| checked[*index])
                        .map(|(_, key)| *key)
                        .collect::<Vec<_>>()
                        .join(",");
                    pairs.push(("ranges".to_string(), subset));
                }
                let order = sort_order.get();
                if !order.is_empty() {
                    let serialized = order
                        .iter()
                        .map(|(key, direction)| format!("{key}:{direction}"))
                        .collect::<Vec<_>>()
                        .join(",");
                    pairs.push(("sort".to_string(), serialized));
                }
                let from = from_epoch.get();
                if !from.is_empty() {
                    pairs.push(("from".to_string(), from));
                }
                let to = to_epoch.get();
                if !to.is_empty() {
                    pairs.push(("to".to_string(), to));
                }
            }
            run_search(pairs, is_search);
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
        move |event: web_sys::Event| {
            let value = event_target_value(&event);
            from_time.set(value.clone());
            from_epoch.set(
                datetime_local_to_epoch_secs(&value)
                    .map(|secs| secs.to_string())
                    .unwrap_or_default(),
            );
            trigger_search();
        }
    };
    let on_to_change = {
        let trigger_search = trigger_search.clone();
        move |event: web_sys::Event| {
            let value = event_target_value(&event);
            to_time.set(value.clone());
            to_epoch.set(
                datetime_local_to_epoch_secs(&value)
                    .map(|secs| secs.to_string())
                    .unwrap_or_default(),
            );
            trigger_search();
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
            from_epoch.get(),
            to_epoch.get(),
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
        <form on:submit=on_submit>
            <div>
                <input
                    type="text"
                    placeholder="search text (space separated words = AND, empty = all)"
                    prop:value=q_filter
                    on:input=move |event| q_filter.set(event_target_value(&event))
                />
            </div>
            <div>
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
            <div>
                <input
                    type="datetime-local"
                    prop:value=from_time
                    on:change=on_from_change
                />
                <input
                    type="datetime-local"
                    prop:value=to_time
                    on:change=on_to_change
                />
            </div>
            <div>
                {SORT_KEYS
                    .iter()
                    .map(|key| {
                        let on_add_sort = on_add_sort.clone();
                        let key = key.to_string();
                        let label = sort_label(&key).to_string();
                        view! {
                            <button type="button" on:click=move |_| on_add_sort(key.clone())>
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
                                <span>
                                    <button type="button" on:click=move |_| on_toggle_dir(toggle_key.clone())>
                                        {arrow}
                                    </button>
                                    {label}
                                    <button type="button" on:click=move |_| on_remove_sort(remove_key.clone())>
                                        {"×"}
                                    </button>
                                </span>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <div>
                <button type="submit" disabled=move || fetching.get()>search</button>
            </div>
        </form>
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
            if !loaded.get() {
                view! { <p>loading...</p> }.into_any()
            } else {
                let pagination = view! {
                    <Pagination
                        current=move || current_page.get()
                        total_pages=move || total_pages.get()
                        on_go=on_go
                    />
                };
                match mode.get() {
                    PageMode::List => {
                        let Some(page) = list_page.get() else {
                            return view! { <p>loading...</p> }.into_any();
                        };
                        if page.article_list.is_empty() {
                            view! { <p>none</p> {pagination} }.into_any()
                        } else {
                            let rows = page
                                .article_list
                                .into_iter()
                                .map(|article| {
                                    let detail_url =
                                        format!("/public/article/{}", article.id);
                                    let tags = article
                                        .tags
                                        .iter()
                                        .map(|tag| tag.name.clone())
                                        .collect::<Vec<_>>()
                                        .join(" · ");
                                    let meta = vec![
                                        article.author_name.clone(),
                                        tags,
                                        article.latest_version.clone(),
                                    ]
                                    .into_iter()
                                    .filter(|part| !part.is_empty())
                                    .collect::<Vec<_>>()
                                    .join(" · ");
                                    view! {
                                        <div>
                                            <div><A href=detail_url>{article.title}</A></div>
                                            <p>{article.summary}</p>
                                            <p>{meta}</p>
                                        </div>
                                    }
                                })
                                .collect_view();
                            view! {
                                <div>
                                    {rows}
                                    {pagination}
                                </div>
                            }
                            .into_any()
                        }
                    }
                    PageMode::Search => {
                        let list = search_list.get();
                        let terms: Vec<String> = q_filter
                            .get()
                            .split_whitespace()
                            .map(|word| word.to_string())
                            .collect();
                        if list.is_empty() {
                            view! { <p>none</p> {pagination} }.into_any()
                        } else {
                            let rows = list
                                .into_iter()
                                .map(|article| {
                                    let detail_url = format!("/public/article/{}", article.id);
                                    let header = format!(
                                        "{} · {} · {}",
                                        article.title, article.author, article.time
                                    );
                                    let hits = article.hits.clone();
                                    view! {
                                        <div>
                                            <div><A href=detail_url>{header}</A></div>
                                            {hits
                                                .into_iter()
                                                .map(|hit| {
                                                    let snippet_html =
                                                        render_snippet(&hit.snippet, &terms);
                                                    view! {
                                                        <div>
                                                            <span>{format!("[{}]", hit.label)}</span>
                                                            <span inner_html=snippet_html></span>
                                                        </div>
                                                    }
                                                })
                                                .collect_view()}
                                        </div>
                                    }
                                })
                                .collect_view();
                            view! {
                                <div>
                                    {rows}
                                    {pagination}
                                </div>
                            }
                            .into_any()
                        }
                    }
                }
            }
        }}
    }
}
