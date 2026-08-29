use std::future::Future;

use common::response::ListPage;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::query_signal_with_options;

use crate::page::fetch::{LoadError, notify_load_failures};

pub fn total_pages(total: u64, per_page: u64) -> u64 {
    total.div_ceil(per_page).max(1)
}

pub fn current_page(raw: Option<u64>) -> u64 {
    raw.unwrap_or(1).max(1)
}

pub fn clamp_page(target: u64, pages: u64) -> u64 {
    target.clamp(1, pages)
}

#[component]
pub fn PagedLinks<T, L, R, LF>(
    per_page: Signal<u64>,
    label: &'static str,
    empty_message: &'static str,
    load: L,
    render: R,
) -> impl IntoView
where
    T: Send + Sync + Clone + 'static,
    L: Fn(u64, u64) -> LF + Send + Sync + 'static,
    LF: Future<Output = Result<ListPage<T>, LoadError>> + 'static,
    R: Fn(&T) -> AnyView + Send + Sync + 'static,
{
    let (page, set_page) = query_signal_with_options::<u64>(
        "page",
        NavigateOptions {
            replace: true,
            ..Default::default()
        },
    );

    let results: LocalResource<Result<ListPage<T>, LoadError>> = LocalResource::new(move || {
        let page_number = current_page(page.get());
        let page_size = per_page.get().max(1);
        load(page_number, page_size)
    });
    notify_load_failures(results);

    let current = Signal::derive(move || current_page(page.get()));
    let pages = Signal::derive(move || {
        results.get().map_or(0, |result| {
            result.map_or(0, |list| total_pages(list.total, per_page.get().max(1)))
        })
    });
    let has_prev = Signal::derive(move || current.get() > 1);
    let has_next = Signal::derive(move || {
        results
            .get()
            .is_some_and(|result| result.is_ok_and(|list| list.has_next))
    });
    let on_go = Callback::new(move |target: u64| {
        if target == current.get_untracked() {
            return;
        }
        set_page.set(Some(clamp_page(target, pages.get_untracked().max(1))));
    });

    view! {
        <div class="mx-auto w-full max-w-2xl px-6 py-8">
            <Suspense fallback=|| view! { <p class="text-muted">"Loading..."</p> }>
                {move || match results.get() {
                    Some(Ok(list)) => {
                        let header = view! {
                            <div class="mb-5 flex items-baseline justify-between gap-4">
                                <h1 class="text-2xl font-semibold tracking-tight text-ink">
                                    {format!("{} {}", list.total, label)}
                                </h1>
                                <span class="text-sm text-muted">{move || format!("page {} of {}", current.get(), pages.get())}</span>
                            </div>
                        };
                        let body = if list.items.is_empty() {
                            view! { <p class="rounded-xl border border-line bg-card px-5 py-6 text-center text-muted">{empty_message}</p> }
                                .into_any()
                        } else {
                            let filler = usize::try_from(per_page.get().max(1))
                                .unwrap_or(usize::MAX)
                                .saturating_sub(list.items.len());
                            view! { <ul class="divide-y divide-line overflow-hidden rounded-xl border border-line-strong bg-card shadow-sm">
                                {list.items.iter().map(|item| {
                                    let inner = render(item);
                                    view! { <li class="px-5 py-3">{inner}</li> }
                                }).collect_view()}
                                {(0..filler).map(|_| {
                                    view! { <li class="px-5 py-3" aria-hidden="true">{"\u{00a0}"}</li> }
                                }).collect_view()}
                            </ul> }
                                .into_any()
                        };
                        view! { {header} {body} <PagedControls current=current pages=pages has_prev=has_prev has_next=has_next on_go=on_go /> }
                            .into_any()
                    }
                    Some(Err(message)) => view! { <p class="rounded-xl border border-line bg-card px-5 py-6 text-center text-muted">{message.to_string()}</p> }
                        .into_any(),
                    None => view! { <p class="text-muted">"Loading..."</p> }.into_any(),
                }}
            </Suspense>
        </div>
    }
}

fn commit_input(
    page_input: RwSignal<String>,
    current: Signal<u64>,
    pages: Signal<u64>,
    on_go: Callback<u64>,
) {
    let Ok(target) = page_input.get_untracked().parse::<u64>() else {
        page_input.set(current.get_untracked().to_string());
        return;
    };
    let target = clamp_page(target, pages.get_untracked().max(1));
    if target == current.get_untracked() {
        page_input.set(target.to_string());
    } else {
        on_go.run(target);
    }
}

#[component]
fn PagedControls(
    #[prop(into)] current: Signal<u64>,
    #[prop(into)] pages: Signal<u64>,
    #[prop(into)] has_prev: Signal<bool>,
    #[prop(into)] has_next: Signal<bool>,
    on_go: Callback<u64>,
) -> impl IntoView {
    let page_input = RwSignal::new(current.get_untracked().to_string());
    Effect::new(move |_| page_input.set(current.get().to_string()));

    let on_submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        commit_input(page_input, current, pages, on_go);
    };
    let on_change = move |_event: web_sys::Event| {
        commit_input(page_input, current, pages, on_go);
    };
    let on_input = move |event: web_sys::Event| page_input.set(event_target_value(&event));

    view! {
        <form class="pagination" on:submit=on_submit>
            <button
                type="button"
                on:click=move |_| on_go.run(current.get().saturating_sub(1).max(1))
                disabled=move || !has_prev.get()
            >"prev"</button>
            <input
                type="number"
                min="1"
                max=move || pages.get().max(1)
                prop:value=page_input
                on:input=on_input
                on:change=on_change
            />
            <span class="total">{move || format!("/ {}", pages.get())}</span>
            <button
                type="button"
                on:click=move |_| on_go.run((current.get() + 1).min(pages.get().max(1)))
                disabled=move || !has_next.get()
            >"next"</button>
        </form>
    }
}

#[cfg(test)]
#[path = "paged_links_tests.rs"]
mod tests;
