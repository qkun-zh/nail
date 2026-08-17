use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};
use nail_common::response::search::SearchArticleItem;

use crate::infrastructure::limits::use_limits;
use crate::page::notify::{notify_error, use_notifications};
use crate::page::pagination::PrevNext;
use crate::request::url::encode_component;

mod comments;
mod versions;

use versions::SearchVersions;

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

#[component]
pub fn Search() -> impl IntoView {
    let notifications = use_notifications();
    let navigate = use_navigate();
    let query = use_query_map();
    let limits = use_limits();

    let search_list = RwSignal::new(Vec::<SearchArticleItem>::new());
    let loaded = RwSignal::new(false);
    let fetching = RwSignal::new(false);
    let has_next = RwSignal::new(false);

    let q_filter = RwSignal::new(String::new());
    let ranges = RwSignal::new(vec![true; 7]);
    let from_time = RwSignal::new(String::new());
    let to_time = RwSignal::new(String::new());
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
                    has_next.set(page.has_next);
                    current_page.set(page.page);
                    last_good_page.set_value(page.page);
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
                has_next.set(false);
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
            current_page.get(),
        );
        if previous.is_none() {
            return;
        }
        sync_url();
    });

    do_search.clone()(page);

    view! {
        <div class="searchbar">
            <div class="searchbar-inner">
                <form on:submit=on_submit>
                    <div class="query-row">
                        <input
                            type="text"
                            placeholder="words = OR; +word = must, -word = exclude, \"a b\" = phrase"
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
                    </div>
                </form>
            </div>
        </div>
        <div class="wrap">
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
                        let has_prev = move || current_page.get() > 1;
                        view! {
                            <div>
                                {rows}
                                <PrevNext
                                    current=move || current_page.get()
                                    has_prev=has_prev
                                    has_next=move || has_next.get()
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
