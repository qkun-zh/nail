use common::request::DeleteMode;
use leptos::prelude::*;

pub const ALL_MODES: [DeleteMode; 3] = [DeleteMode::Transfer, DeleteMode::Soft, DeleteMode::Hard];
pub const SOFT_AND_HARD: [DeleteMode; 2] = [DeleteMode::Soft, DeleteMode::Hard];

pub fn mode_to_str(mode: DeleteMode) -> &'static str {
    mode.as_str()
}

pub fn mode_from_str(value: &str, allowed: &[DeleteMode]) -> Option<DeleteMode> {
    allowed.iter().copied().find(|&mode| mode.as_str() == value)
}

#[component]
pub fn DeleteModePicker(
    mode: RwSignal<DeleteMode>,
    name: &'static str,
    allowed: &'static [DeleteMode],
) -> impl IntoView {
    allowed
        .iter()
        .map(|&allowed_mode| {
            let is_selected = move || mode.get() == allowed_mode;
            view! {
                <div>
                    <label>
                        <input
                            type="radio"
                            name=name
                            prop:checked=is_selected
                            on:change=move |_| mode.set(allowed_mode)
                        />
                        {mode_to_str(allowed_mode)}
                    </label>
                </div>
            }
        })
        .collect_view()
}

#[cfg(test)]
#[path = "delete_mode_tests.rs"]
mod tests;
