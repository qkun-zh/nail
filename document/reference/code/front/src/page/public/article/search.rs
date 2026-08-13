use crate::limits::use_limits;
use crate::page::Pagination;
use crate::page::auth_gate::use_component_alive;
use crate::page::notify::{notify_error, use_notify};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::{use_location, use_navigate};

const RANGE_KEYS: [&str; 6] = ["title", "summary", "author", "comment", "note", "tag"];
const RANGE_LABELS: [&str; 6] = ["标题", "摘要", "作者", "评论", "版本说明", "标签"];
const SORT_KEYS: [&str; 3] = ["time", "title", "author"];

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn datetime_local_to_epoch_secs(value: &str) -> Option<u64> {
    if value.is_empty() { return None; }
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from(value));
    let millis = date.get_time();
    if millis.is_nan() || millis < 0.0 { return None; }
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
    if value.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(secs) = value.parse::<u64>() {
            return epoch_secs_to_datetime_local(secs);
        }
    }
    value.to_string()
}

fn sort_label(key: &str) -> &str {
    match key {
        "time" => "时间",
        "title" => "标题字母序",
        "author" => "作者名字母序",
        _ => key,
    }
}

fn default_sort_dir(key: &str) -> String {
    if key == "time" { "desc".to_string() } else { "asc".to_string() }
}

fn dir_arrow(dir: &str) -> &'static str {
    if dir == "desc" { "↓" } else { "↑" }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn highlight_terms(escaped: &str, terms: &[String]) -> String {
    let lower = escaped.to_lowercase();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for term in terms {
        let needle = escape_html(term).to_lowercase();
        if needle.is_empty() { continue; }
        let mut start = 0;
        while let Some(pos) = lower[start..].find(&needle) {
            let abs = start + pos;
            spans.push((abs, abs + needle.len()));
            start = abs + needle.len();
        }
    }
    spans.sort_by_key(|a| a.0);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in spans {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 { last.1 = last.1.max(e); continue; }
        }
        merged.push((s, e));
    }
    let mut out = String::new();
    let mut cursor = 0;
    for (s, e) in merged {
        out.push_str(&escaped[cursor..s]);
        out.push_str("<mark>");
        out.push_str(&escaped[s..e]);
        out.push_str("</mark>");
        cursor = e;
    }
    out.push_str(&escaped[cursor..]);
    out
}

fn render_snippet(snippet: &str, terms: &[String]) -> String {
    if terms.is_empty() { escape_html(snippet) } else { highlight_terms(&escape_html(snippet), terms) }
}

#[component]
pub fn Search() -> impl IntoView {
    let notification = use_notify();
    let location = use_location();
    let navigate = use_navigate();
    let alive = use_component_alive();

    let article_list = RwSignal::new(Vec::<serde_json::Value>::new());
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
    let limits = use_limits();
    let per_page = RwSignal::new(limits.get().search_page_size as u64);

    let params = location.query.get_untracked();
    q_filter.set(params.get("q").unwrap_or_default());
    if let Some(r) = params.get("ranges") {
        let mut v = vec![false; 6];
        if !r.is_empty() {
            for (i, key) in RANGE_KEYS.iter().enumerate() {
                if r.split(',').any(|piece| piece == *key) { v[i] = true; }
            }
        }
        ranges.set(v);
    }
    if let Some(s) = params.get("sort") {
        let mut order = Vec::new();
        for piece in s.split(',') {
            let mut it = piece.splitn(2, ':');
            let key = it.next().unwrap_or("");
            let default_dir = default_sort_dir(key);
            let dir = it.next().unwrap_or(&default_dir);
            if SORT_KEYS.iter().any(|k| *k == key) {
                order.push((key.to_string(), dir.to_string()));
            }
        }
        sort_order.set(order);
    }
    from_time.set(params.get("from").map(|s| url_time_to_local(&s)).unwrap_or_default());
    to_time.set(params.get("to").map(|s| url_time_to_local(&s)).unwrap_or_default());
    from_epoch.set(params.get("from").unwrap_or_default());
    to_epoch.set(params.get("to").unwrap_or_default());
    let page = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
    current_page.set(page);
    if let Some(l) = params.get("limit").and_then(|l| l.parse().ok()) {
        per_page.set(l);
    }

    let pathname = location.pathname.get_untracked();
    let sync_url = {
        let navigate = navigate.clone();
        let pathname = pathname.clone();
        move || {
            let mut pairs: Vec<String> = Vec::new();
            let q = q_filter.get();
            if !q.trim().is_empty() {
                pairs.push(format!("q={}", crate::req::url_encode(q.trim())));
            }
            let r = ranges.get();
            if !r.iter().all(|&c| c) {
                let subset = RANGE_KEYS.iter().enumerate()
                    .filter(|(i, _)| r[*i]).map(|(_, k)| *k)
                    .collect::<Vec<_>>().join(",");
                pairs.push(format!("ranges={}", crate::req::url_encode(&subset)));
            }
            let order = sort_order.get();
            if !order.is_empty() {
                let s = order.iter().map(|(k, d)| format!("{k}:{d}"))
                    .collect::<Vec<_>>().join(",");
                pairs.push(format!("sort={}", crate::req::url_encode(&s)));
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
            navigate(&format!("{pathname}?{query_string}"),
                leptos_router::NavigateOptions { replace: true, resolve: false, ..Default::default() });
        }
    };

    let build_query = move |page: u64| {
        let r = ranges.get();
        let ranges_param = if r.iter().all(|&c| c) {
            None
        } else {
            Some(RANGE_KEYS.iter().enumerate().filter(|(i, _)| r[*i])
                .map(|(_, k)| *k).collect::<Vec<_>>().join(","))
        };
        let order = sort_order.get();
        let sort_param = if order.is_empty() {
            None
        } else {
            Some(order.iter().map(|(k, d)| format!("{k}:{d}")).collect::<Vec<_>>().join(","))
        };
        Some(crate::req::ArticleSearchParams {
            q: non_empty_string(&q_filter.get()),
            ranges: ranges_param,
            sort: sort_param,
            from: from_epoch.get().parse::<u64>().ok(),
            to: to_epoch.get().parse::<u64>().ok(),
            limit: Some(per_page.get()),
            page: Some(page),
        })
    };

    let request_seq = StoredValue::new(0u64);
    let last_good_page = StoredValue::new(1u64);
    let run_search = {
        move |query: crate::req::ArticleSearchParams| {
            let my_seq = request_seq.get_value() + 1;
            request_seq.set_value(my_seq);
            fetching.set(true);
            spawn_local({
                let alive = alive.clone();
                async move {
                    match crate::req::search_articles(&query).await {
                        Ok(data) => {
                            if !alive.get_value() || request_seq.get_value() != my_seq { return; }
                            let list = data.get("article_list").and_then(|a| a.as_array()).cloned().unwrap_or_default();
                            total.set(data.get("total").and_then(|v| v.as_u64()).unwrap_or(0));
                            let pages = data.get("total_pages").and_then(|v| v.as_u64()).unwrap_or(0);
                            total_pages.set(pages);
                            truncated.set(data.get("truncated").and_then(|v| v.as_bool()).unwrap_or(false));
                            let served = data.get("page").and_then(|v| v.as_u64()).unwrap_or(1);
                            let committed = if pages > 0 { served.min(pages) } else { served };
                            current_page.set(committed);
                            last_good_page.set_value(committed);
                            article_list.set(list);
                            loaded.set(true);
                            fetching.set(false);
                        }
                        Err(e) => {
                            if !alive.get_value() { return; }
                            if request_seq.get_value() == my_seq {
                                notify_error(&notification, &format!("search failed: {e}"));
                                current_page.set(last_good_page.get_value());
                                loaded.set(true);
                                fetching.set(false);
                            }
                        }
                    }
                }
            });
        }
    };

    let do_search = move |page: u64| -> bool {
        let Some(query) = build_query(page) else { return false; };
        run_search(query);
        true
    };

    let on_go = Callback::new({
        let do_search = do_search.clone();
        move |target: u64| {
            let current_page_value = current_page.get();
            if target == current_page_value { return; }
            if do_search(target) { current_page.set(target); }
        }
    });

    Effect::new(move |prev: Option<()>| {
        let _ = (q_filter.get(), ranges.get(), from_time.get(), to_time.get(), from_epoch.get(), to_epoch.get(), sort_order.get(), current_page.get());
        if prev.is_none() { return; }
        sync_url();
    });

    do_search.clone()(page);

    let trigger_search = {
        let do_search = do_search.clone();
        move || { if do_search(1) { current_page.set(1); } }
    };
    let on_submit = {
        let trigger_search = trigger_search.clone();
        move |ev: SubmitEvent| { ev.prevent_default(); trigger_search(); }
    };
    let on_range_change = {
        let trigger_search = trigger_search.clone();
        move |idx: usize, ev: web_sys::Event| {
            ranges.update(|v| v[idx] = event_target_checked(&ev));
            trigger_search();
        }
    };
    let on_add_sort = {
        let do_search = do_search.clone();
        move |key: String| {
            sort_order.update(|list| {
                if !list.iter().any(|(k, _)| *k == key) {
                    list.push((key.clone(), default_sort_dir(&key)));
                }
            });
            if do_search(1) { current_page.set(1); }
        }
    };
    let on_toggle_dir = {
        let do_search = do_search.clone();
        move |key: String| {
            sort_order.update(|list| {
                if let Some(entry) = list.iter_mut().find(|(k, _)| *k == key) {
                    entry.1 = if entry.1 == "asc" { "desc".to_string() } else { "asc".to_string() };
                }
            });
            if do_search(1) { current_page.set(1); }
        }
    };
    let on_remove_sort = {
        let do_search = do_search.clone();
        move |key: String| {
            sort_order.update(|list| list.retain(|(k, _)| *k != key));
            if do_search(1) { current_page.set(1); }
        }
    };
    let on_from_change = {
        let do_search = do_search.clone();
        move |ev: web_sys::Event| {
            let val = event_target_value(&ev);
            from_time.set(val.clone());
            from_epoch.set(
                datetime_local_to_epoch_secs(&val)
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            );
            if do_search(1) { current_page.set(1); }
        }
    };
    let on_to_change = {
        let do_search = do_search.clone();
        move |ev: web_sys::Event| {
            let val = event_target_value(&ev);
            to_time.set(val.clone());
            to_epoch.set(
                datetime_local_to_epoch_secs(&val)
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            );
            if do_search(1) { current_page.set(1); }
        }
    };

    view! {
        <form on:submit=on_submit>
            <div><input type="text" placeholder="任意文本…（多词 = AND，空 = 全部）" bind:value=q_filter/></div>
            <div>
                {RANGE_LABELS.iter().enumerate().map(|(i, label)| {
                    let handler = {
                        let on_range_change = on_range_change.clone();
                        move |ev: web_sys::Event| on_range_change(i, ev)
                    };
                    let checked = move || ranges.get()[i];
                    view! { <label><input type="checkbox" prop:checked=checked on:change=handler/>{*label}</label> }
                }).collect::<Vec<_>>()}
            </div>
            <div>
                <input type="datetime-local" bind:value=from_time on:change=on_from_change/>
                <input type="datetime-local" bind:value=to_time on:change=on_to_change/>
            </div>
            <div>
                {SORT_KEYS.iter().map(|key| {
                    let on_add_sort = on_add_sort.clone();
                    let key = key.to_string();
                    let label = sort_label(&key).to_string();
                    view! { <button type="button" on:click=move |_| on_add_sort(key.clone())>{label}</button> }
                }).collect::<Vec<_>>()}
                {move || {
                    let order = sort_order.get();
                    order.into_iter().map(|(key, dir)| {
                        let on_toggle_dir = on_toggle_dir.clone();
                        let on_remove_sort = on_remove_sort.clone();
                        let toggle_key = key.clone();
                        let remove_key = key.clone();
                        let label = sort_label(&key).to_string();
                        let arrow = dir_arrow(&dir).to_string();
                        view! {
                            <span><button type="button" on:click=move |_| on_toggle_dir(toggle_key.clone())>{arrow}</button>{label}<button type="button" on:click=move |_| on_remove_sort(remove_key.clone())>{"×"}</button></span>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>
            <div><button type="submit" disabled={move || fetching.get()}>search</button></div>
        </form>
        {move || if truncated.get() {
            let msg = format!("too many results ({} records) - only the first {} pages are shown, add more conditions to narrow down", total.get(), limits.get().max_search_pages);
            view! { <p>{msg}</p> }.into_any()
        } else { ().into_any() }}
        {move || if !loaded.get() {
            view! { <p>loading...</p> }.into_any()
        } else {
            let list = article_list.get();
            let terms: Vec<String> = q_filter.get().split_whitespace().map(|s| s.to_string()).collect();
            let pagination = view! {
                <Pagination current=move || current_page.get() total_pages=move || total_pages.get() on_go=on_go/>
            };
            if list.is_empty() {
                view! { <p>none</p> {pagination} }.into_any()
            } else {
                view! {
                    <div>
                        {list.into_iter().map(|article| {
                            let id = article.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let title = article.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let author = article.get("author").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let time = article.get("time").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let detail_url = format!("/public/article/{}", crate::req::url_encode(&id));
                            let hits = article.get("hits").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                            let header = format!("{} · {} · {}", title, author, time);
                            view! {
                                <div>
                                    <div><A href=detail_url>{header}</A></div>
                                    {hits.into_iter().map(|hit| {
                                        let label = hit.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let snippet = hit.get("snippet").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let snippet_html = render_snippet(&snippet, &terms);
                                        view! { <div><span>{format!("[{}]", label)}</span><span inner_html=snippet_html></span></div> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                    {pagination}
                }.into_any()
            }
        }}
    }
}
