use leptos::prelude::*;

#[component]
pub fn Pagination(
    #[prop(into)]
    current: Signal<u64>,
    #[prop(into)]
    total_pages: Signal<u64>,
    on_go: Callback<u64>,
    #[prop(optional, into)]
    has_prev: Option<Signal<bool>>,
    #[prop(optional, into)]
    has_more: Option<Signal<bool>>,
) -> impl IntoView {
    let page_input = RwSignal::new(current.get_untracked().to_string());
    Effect::new(move |_| page_input.set(current.get().to_string()));
    let prev_disabled = move || match has_prev {
        Some(s) => !s.get(),
        None => current.get() <= 1,
    };
    let next_disabled = move || match has_more {
        Some(s) => !s.get(),
        None => current.get() >= total_pages.get(),
    };
    let commit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
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
                <button type="button" on:click=move |_| on_go.run(current.get().saturating_sub(1).max(1)) disabled=prev_disabled>prev</button>
                <input type="number" min="1" max=move || total_pages.get().to_string() bind:value=page_input/>
                <span>{move || format!("/ {}", total_pages.get())}</span>
                <button type="button" on:click=move |_| on_go.run((current.get() + 1).min(total_pages.get())) disabled=next_disabled>next</button>
            </form>
        }
        .into_any()
    }
}
