use leptos::prelude::*;

const MAX_PAGE_SIZE: u64 = 200;

pub fn clamp_page_size(limit: u64, fallback: u64) -> u64 {
    if limit == 0 {
        fallback
    } else {
        limit.clamp(1, MAX_PAGE_SIZE)
    }
}

pub fn local_page_count(item_count: usize, per_page: u64) -> u64 {
    (item_count as u64).div_ceil(per_page).max(1)
}

pub fn clamp_local_page(target: u64, page_count: u64) -> u64 {
    target.clamp(1, page_count)
}

pub fn local_page_of(index: usize, per_page: u64) -> u64 {
    (index as u64) / per_page
}

/// A client-side paged list whose data is already fully present in memory.
/// Paging is local state only (no request, no URL); refreshing returns to the
/// first page. `render` maps each item on the current page to a view.
#[component]
pub fn LocalPagedList<T, R>(
    items: Vec<T>,
    per_page: u64,
    render: R,
    #[prop(optional)] pagination_class: Option<&'static str>,
) -> impl IntoView
where
    T: Send + Sync + 'static,
    R: Fn(&T) -> leptos::prelude::AnyView + Send + Sync + 'static,
{
    let current = RwSignal::new(1u64);
    let total_pages = local_page_count(items.len(), per_page);
    let on_go = Callback::new(move |target: u64| {
        current.set(clamp_local_page(target, total_pages));
    });
    let page_list = move || {
        let page = current.get();
        items
            .iter()
            .enumerate()
            .filter(|(index, _)| local_page_of(*index, per_page) == page - 1)
            .map(|(_, item)| render(item))
            .collect::<Vec<_>>()
    };
    view! {
        {page_list}
        {move || {
            if total_pages > 1 {
                view! {
                    <Pagination
                        current=current
                        total_pages=move || total_pages
                        on_go=on_go
                        pagination_class=pagination_class.unwrap_or("pagination")
                    />
                }
                .into_any()
            } else {
                ().into_any()
            }
        }}
    }
}

#[component]
pub fn Pagination(
    #[prop(into)] current: Signal<u64>,
    #[prop(into)] total_pages: Signal<u64>,
    on_go: Callback<u64>,
    #[prop(optional, into)] has_prev: Option<Signal<bool>>,
    #[prop(optional, into)] has_more: Option<Signal<bool>>,
    #[prop(optional)] pagination_class: Option<&'static str>,
) -> impl IntoView {
    let page_input = RwSignal::new(current.get_untracked().to_string());
    Effect::new(move |_| page_input.set(current.get().to_string()));
    let prev_disabled = move || match has_prev {
        Some(signal) => !signal.get(),
        None => current.get() <= 1,
    };
    let next_disabled = move || match has_more {
        Some(signal) => !signal.get(),
        None => current.get() >= total_pages.get(),
    };
    let commit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let Ok(page) = page_input.get_untracked().parse::<u64>() else {
            page_input.set(current.get_untracked().to_string());
            return;
        };
        let total = total_pages.get_untracked().max(1);
        let clamped = page.clamp(1, total);
        if clamped == current.get_untracked() {
            page_input.set(clamped.to_string());
        } else {
            on_go.run(clamped);
        }
    };
    let pagination_class = pagination_class.unwrap_or("pagination");
    move || {
        if total_pages.get() == 0 {
            return ().into_any();
        }
        view! {
            <form class=pagination_class on:submit=commit>
                <button
                    type="button"
                    on:click=move |_| on_go.run(current.get().saturating_sub(1).max(1))
                    disabled=prev_disabled
                >prev</button>
                <input
                    type="number"
                    min="1"
                    max=move || total_pages.get().to_string()
                    prop:value=page_input
                    on:input=move |event| page_input.set(event_target_value(&event))
                />
                <span class="total">{move || format!("/ {}", total_pages.get())}</span>
                <button
                    type="button"
                    on:click=move |_| on_go.run((current.get() + 1).min(total_pages.get()))
                    disabled=next_disabled
                >next</button>
            </form>
        }
        .into_any()
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/front/page/pagination/tests.rs"]
mod tests;
