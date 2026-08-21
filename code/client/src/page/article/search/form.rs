use leptos::ev::SubmitEvent;
use leptos::prelude::*;

use super::RANGE_SPECS;

#[component]
pub(super) fn SearchForm(
    on_submit: Callback<SubmitEvent>,
    q_filter: RwSignal<String>,
    fetching: RwSignal<bool>,
    ranges: RwSignal<Vec<bool>>,
    on_range_change: Callback<(usize, web_sys::Event)>,
    from_time: RwSignal<String>,
    on_from_change: Callback<web_sys::Event>,
    to_time: RwSignal<String>,
    on_to_change: Callback<web_sys::Event>,
) -> impl IntoView {
    view! {
        <div class="searchbar">
            <div class="searchbar-inner">
                <form on:submit=move |event: SubmitEvent| on_submit.run(event)>
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
                            {RANGE_SPECS
                                .iter()
                                .enumerate()
                                .map(|(index, spec)| {
                                    let handler = move |event: web_sys::Event| {
                                        on_range_change.run((index, event));
                                    };
                                    let checked = move || ranges.get()[index];
                                    view! {
                                        <label>
                                            <input type="checkbox" prop:checked=checked on:change=handler/>
                                            {spec.label}
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
                                on:change=move |event: web_sys::Event| on_from_change.run(event)
                            />
                            <input
                                type="text"
                                placeholder="to (ISO8601, UTC)"
                                prop:value=to_time
                                on:change=move |event: web_sys::Event| on_to_change.run(event)
                            />
                        </div>
                    </div>
                </form>
            </div>
        </div>
    }
}
