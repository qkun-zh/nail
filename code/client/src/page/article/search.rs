use common::response::search::SearchArticleItem;
use common::search::SearchRange;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::{query_signal_with_options, use_query_map};

use crate::infrastructure::limits::use_limits;
use crate::page::fetch::{LoadError, Loaded};
use crate::page::notify::{notify_error, use_notifications};

mod comments;
mod form;
mod results;
mod versions;

use form::SearchForm;
use results::SearchResults;

fn range_label(range: SearchRange) -> &'static str {
    match range {
        SearchRange::AuthorName => "author name",
        SearchRange::Note => "version note",
        SearchRange::VersionNumber => "version number",
        SearchRange::ArticleId => "article id",
        SearchRange::VersionId => "version id",
        SearchRange::CommentId => "comment id",
        SearchRange::AuthorId => "author id",
        _ => range.label(),
    }
}
fn checked_range_subset(checked: &[bool]) -> String {
    SearchRange::ALL
        .iter()
        .enumerate()
        .filter(|(index, _)| checked[*index])
        .map(|(_, range)| range.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

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

/// Latest successfully rendered search result set.
#[derive(Clone)]
struct SearchOutcome {
    items: Vec<SearchArticleItem>,
    has_next: bool,
    page: u64,
}

#[component]
pub fn Search() -> impl IntoView {
    let notifications = use_notifications();
    let query = use_query_map();
    let limits = use_limits();

    let search_list = RwSignal::new(Vec::<SearchArticleItem>::new());
    let loaded = RwSignal::new(false);
    let fetching = RwSignal::new(false);
    let has_next = RwSignal::new(false);

    let q_filter = RwSignal::new(String::new());
    let ranges = RwSignal::new(vec![true; 12]);
    let from_time = RwSignal::new(String::new());
    let to_time = RwSignal::new(String::new());
    let current_page = RwSignal::new(1u64);
    let per_page = RwSignal::new(limits.get_untracked().search_page_size);

    let params = query.get_untracked();
    q_filter.set(params.get("q").unwrap_or_default());
    if let Some(ranges_param) = params.get("ranges") {
        let mut checked = vec![false; 12];
        if !ranges_param.is_empty() {
            for (index, range) in SearchRange::ALL.iter().enumerate() {
                if ranges_param.split(',').any(|piece| piece == range.as_str()) {
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
        .unwrap_or(1)
        .max(1);
    current_page.set(page);
    if let Some(limit) = params
        .get("limit")
        .and_then(|value| value.parse::<u64>().ok())
    {
        per_page.set(limit);
    }

    // Explicit triggers bump the epoch; field signals are read untracked at fetch
    // time so typing alone never fires a request. The resource itself serializes
    // concurrent runs, replacing the hand-rolled sequence counter.
    let epoch = RwSignal::new(0u64);
    let requested_page = RwSignal::new(current_page.get_untracked());

    let results: LocalResource<Loaded<Option<SearchOutcome>>> = LocalResource::new(move || {
        let _token = epoch.get();
        let requested = requested_page.get();
        let mut pairs: Vec<(String, String)> = vec![
            ("page".to_string(), requested.to_string()),
            ("limit".to_string(), per_page.get_untracked().to_string()),
        ];
        let q = q_filter.get_untracked().trim().to_string();
        if !q.is_empty() {
            pairs.push(("q".to_string(), q.clone()));
            let checked = ranges.get_untracked();
            pairs.push(("ranges".to_string(), checked_range_subset(&checked)));
            let from = from_time.get_untracked();
            if !from.trim().is_empty() {
                pairs.push(("from".to_string(), from.trim().to_string()));
            }
            let to = to_time.get_untracked();
            if !to.trim().is_empty() {
                pairs.push(("to".to_string(), to.trim().to_string()));
            }
        }
        async move {
            if q.is_empty() {
                return Ok(None);
            }
            let borrows: Vec<(&str, &str)> = pairs
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str()))
                .collect();
            let page = crate::request::article::search_articles(&borrows)
                .await
                .map_err(LoadError::from)?;
            Ok(Some(SearchOutcome {
                items: page.items,
                has_next: page.has_next,
                page: requested,
            }))
        }
    });

    // Mirror the newest resolved run into the view signals; failures keep the
    // previous result set visible and only toast.
    let bridge_notifications = notifications.clone();
    Effect::new(move |_| match results.get() {
        None => fetching.set(true),
        Some(Ok(Some(outcome))) => {
            fetching.set(false);
            loaded.set(true);
            search_list.set(outcome.items);
            has_next.set(outcome.has_next);
            current_page.set(outcome.page);
        }
        Some(Ok(None)) => {
            fetching.set(false);
            loaded.set(true);
            search_list.set(Vec::new());
            has_next.set(false);
            current_page.set(1);
        }
        Some(Err(error)) => {
            fetching.set(false);
            loaded.set(true);
            notify_error(&bridge_notifications, error.to_string());
        }
    });

    let trigger_search = move || {
        requested_page.set(1);
        epoch.update(|token| *token += 1);
    };

    // Router-owned URL state for every field. Use replaceState to avoid polluting history.
    let replace = NavigateOptions {
        replace: true,
        ..Default::default()
    };
    let (_, set_q) = query_signal_with_options::<String>("q", replace.clone());
    Effect::new(move |_| {
        let value = q_filter.get();
        set_q.set((!value.trim().is_empty()).then_some(value));
    });
    let (_, set_ranges) = query_signal_with_options::<String>("ranges", replace.clone());
    Effect::new(move |_| {
        let checked = ranges.get();
        set_ranges.set(Some(checked_range_subset(&checked)));
    });
    let (_, set_from) = query_signal_with_options::<String>("from", replace.clone());
    Effect::new(move |_| {
        let value = from_time.get();
        set_from.set((!value.trim().is_empty()).then_some(value));
    });
    let (_, set_to) = query_signal_with_options::<String>("to", replace.clone());
    Effect::new(move |_| {
        let value = to_time.get();
        set_to.set((!value.trim().is_empty()).then_some(value));
    });
    let (_, set_page_param) = query_signal_with_options::<u64>("page", replace);
    Effect::new(move |_| {
        set_page_param.set(Some(current_page.get()));
    });
    let on_submit = Callback::new({
        move |event: SubmitEvent| {
            event.prevent_default();
            trigger_search();
        }
    });
    let on_range_change = Callback::new({
        move |(index, event): (usize, web_sys::Event)| {
            ranges.update(|checked| checked[index] = event_target_checked(&event));
            trigger_search();
        }
    });
    let on_from_change = Callback::new({
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
    });
    let on_to_change = Callback::new({
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
    });

    let on_go = Callback::new(move |target: u64| {
        if target == current_page.get() {
            return;
        }
        requested_page.set(target);
        epoch.update(|token| *token += 1);
    });

    view! {
        <SearchForm
            on_submit=on_submit
            q_filter=q_filter
            fetching=fetching
            ranges=ranges
            on_range_change=on_range_change
            from_time=from_time
            on_from_change=on_from_change
            to_time=to_time
            on_to_change=on_to_change
        />
        <div class="wrap">
            <SearchResults
                list=search_list
                loaded=loaded
                q_filter=q_filter
                current_page=current_page
                has_next=has_next
                on_go=on_go
            />
        </div>
    }
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
