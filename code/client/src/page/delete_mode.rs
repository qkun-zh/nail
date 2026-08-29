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
    view! {
        <div class="mt-4 flex w-auto flex-wrap items-center gap-2">
            {allowed
                .iter()
                .map(|&allowed_mode| {
                    let is_selected = move || mode.get() == allowed_mode;
                    let state_class = move || {
                        if is_selected() {
                            " border-brick bg-primary/70 font-bold shadow-sm"
                        } else {
                            " border-line bg-card hover:border-brick-soft hover:bg-bg-soft"
                        }
                    };
                    view! {
                        <label class=move || {
                            let base = "flex cursor-pointer items-center gap-2 rounded-md border px-4 py-2 font-mono text-[20px] leading-normal text-ink shadow-sm transition-none active:scale-95";
                            format!("{base}{}", state_class())
                        }>
                            <input
                                type="radio"
                                name=name
                                prop:checked=is_selected
                                on:change=move |_| mode.set(allowed_mode)
                            />
                            {mode_to_str(allowed_mode)}
                        </label>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[cfg(test)]
#[path = "delete_mode_tests.rs"]
mod tests;
