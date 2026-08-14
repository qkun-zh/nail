use leptos::prelude::*;

const MAX_PAGE_SIZE: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaginationState {
    pub page: u64,
    pub previous_page: Option<u64>,
    pub next_page: Option<u64>,
}

pub fn pagination_state(page: u64, server_has_next: bool) -> PaginationState {
    let page = page.max(1);
    PaginationState {
        page,
        previous_page: (page > 1).then(|| page - 1),
        next_page: server_has_next.then(|| page + 1),
    }
}

pub fn clamp_page_size(limit: u64, fallback: u64) -> u64 {
    if limit == 0 {
        fallback
    } else {
        limit.clamp(1, MAX_PAGE_SIZE)
    }
}

#[component]
pub fn Pagination(
    #[prop(into)] current: Signal<u64>,
    #[prop(into)] total_pages: Signal<u64>,
    on_go: Callback<u64>,
    #[prop(optional, into)] has_prev: Option<Signal<bool>>,
    #[prop(optional, into)] has_more: Option<Signal<bool>>,
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
        if clamped != current.get_untracked() {
            on_go.run(clamped);
        } else {
            page_input.set(clamped.to_string());
        }
    };
    move || {
        if total_pages.get() == 0 {
            return ().into_any();
        }
        view! {
            <form on:submit=commit>
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
                <span>{move || format!("/ {}", total_pages.get())}</span>
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
